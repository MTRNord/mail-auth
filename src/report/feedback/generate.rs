use flate2::{write::GzEncoder, Compression};
use mail_builder::{headers::HeaderType, MessageBuilder};

use crate::report::{
    ActionDisposition, Alignment, AuthResult, DKIMAuthResult, DKIMResult, DMARCResult, DateRange,
    Disposition, Feedback, Identifier, PolicyEvaluated, PolicyOverride, PolicyOverrideReason,
    PolicyPublished, Record, ReportMetadata, Row, SPFAuthResult, SPFDomainScope, SPFResult,
};

use std::{
    borrow::Cow,
    fmt::{Display, Formatter, Write},
};

impl Feedback {
    pub fn write_rfc5322<'x>(
        &self,
        receiver_domain: &'x str,
        submitter: &'x str,
        from: &'x str,
        to: &'x str,
        writer: impl std::io::Write,
    ) -> std::io::Result<()> {
        // Compress XML report
        let xml = self.as_xnl();
        let mut e = GzEncoder::new(Vec::with_capacity(xml.len()), Compression::default());
        std::io::Write::write_all(&mut e, xml.as_bytes())?;
        let compressed_bytes = e.finish()?;

        MessageBuilder::new()
            .header("From", HeaderType::Text(from.into()))
            .header("To", HeaderType::Text(to.into()))
            .header("Auto-Submitted", HeaderType::Text("auto-generated".into()))
            .subject(format!(
                "Report Domain: {} Submitter: {} Report-ID: <{}>",
                self.domain(),
                submitter,
                self.report_id()
            ))
            .text_body(format!(
                concat!(
                    "DMARC aggregate report from {}\r\n\r\n",
                    "Report Domain: {}\r\n",
                    "Submitter: {}\r\n",
                    "Report-ID: {}\r\n",
                ),
                receiver_domain,
                self.domain(),
                submitter,
                self.report_id()
            ))
            .binary_attachment(
                "application/gzip",
                format!(
                    "{}!{}!{}!{}.xml.gz",
                    receiver_domain,
                    self.domain(),
                    self.date_range_begin(),
                    self.date_range_end()
                ),
                compressed_bytes,
            )
            .write_to(writer)
    }

    pub fn as_rfc5322<'x>(
        &self,
        receiver_domain: &'x str,
        submitter: &'x str,
        from: &'x str,
        to: &'x str,
    ) -> std::io::Result<String> {
        let mut buf = Vec::new();
        self.write_rfc5322(receiver_domain, submitter, from, to, &mut buf)?;
        String::from_utf8(buf).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }

    pub fn as_xnl(&self) -> String {
        let mut xml = String::with_capacity(128);
        writeln!(&mut xml, "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>").ok();
        writeln!(&mut xml, "<feedback>").ok();
        if self.version != 0.0 {
            writeln!(&mut xml, "\t<version>{}</version>", self.version).ok();
        }
        self.report_metadata.as_xml(&mut xml);
        self.policy_published.as_xml(&mut xml);
        for record in &self.record {
            record.as_xml(&mut xml);
        }
        writeln!(&mut xml, "</feedback>").ok();
        xml
    }
}

impl ReportMetadata {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t<report_metadata>").ok();
        writeln!(
            xml,
            "\t\t<org_name>{}</org_name>",
            escape_xml(&self.org_name)
        )
        .ok();
        writeln!(xml, "\t\t<email>{}</email>", escape_xml(&self.email)).ok();
        if let Some(eci) = &self.extra_contact_info {
            writeln!(
                xml,
                "\t\t<extra_contact_info>{}</extra_contact_info>",
                escape_xml(eci)
            )
            .ok();
        }
        writeln!(
            xml,
            "\t\t<report_id>{}</report_id>",
            escape_xml(&self.report_id)
        )
        .ok();
        self.date_range.as_xml(xml);
        for error in &self.error {
            writeln!(xml, "\t\t<error>{}</error>", escape_xml(error)).ok();
        }
        writeln!(xml, "\t</report_metadata>").ok();
    }
}

impl PolicyPublished {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t<policy_published>").ok();
        writeln!(xml, "\t\t<domain>{}</domain>", escape_xml(&self.domain)).ok();
        if let Some(vp) = &self.version_published {
            writeln!(xml, "\t\t<version_published>{}</version_published>", vp).ok();
        }
        writeln!(xml, "\t\t<adkim>{}</adkim>", &self.adkim).ok();
        writeln!(xml, "\t\t<aspf>{}</aspf>", &self.aspf).ok();
        writeln!(xml, "\t\t<p>{}</p>", &self.p).ok();
        writeln!(xml, "\t\t<sp>{}</sp>", &self.sp).ok();
        if self.testing {
            writeln!(xml, "\t\t<testing>y</testing>").ok();
        }
        if let Some(fo) = &self.fo {
            writeln!(xml, "\t\t<fo>{}</fo>", escape_xml(fo)).ok();
        }
        writeln!(xml, "\t</policy_published>").ok();
    }
}

impl DateRange {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t\t<date_range>").ok();
        writeln!(xml, "\t\t\t<begin>{}</begin>", self.begin).ok();
        writeln!(xml, "\t\t\t<end>{}</end>", self.end).ok();
        writeln!(xml, "\t\t</date_range>").ok();
    }
}

impl Record {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t<record>").ok();
        self.row.as_xml(xml);
        self.identifiers.as_xml(xml);
        self.auth_results.as_xml(xml);
        writeln!(xml, "\t</record>").ok();
    }
}

impl Row {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t\t<row>").ok();
        writeln!(xml, "\t\t\t<source_ip>{}</source_ip>", self.source_ip).ok();
        writeln!(xml, "\t\t\t<count>{}</count>", self.count).ok();
        self.policy_evaluated.as_xml(xml);
        writeln!(xml, "\t\t</row>").ok();
    }
}

impl PolicyEvaluated {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t\t\t<policy_evaluated>").ok();
        writeln!(
            xml,
            "\t\t\t\t<disposition>{}</disposition>",
            self.disposition
        )
        .ok();
        writeln!(xml, "\t\t\t\t<dkim>{}</dkim>", self.dkim).ok();
        writeln!(xml, "\t\t\t\t<spf>{}</spf>", self.spf).ok();
        for reason in &self.reason {
            reason.as_xml(xml);
        }
        writeln!(xml, "\t\t\t</policy_evaluated>").ok();
    }
}

impl PolicyOverrideReason {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t\t\t\t<reason>").ok();
        writeln!(xml, "\t\t\t\t\t<type>{}</type>", self.type_).ok();
        if let Some(comment) = &self.comment {
            writeln!(xml, "\t\t\t\t\t<comment>{}</comment>", escape_xml(comment)).ok();
        }
        writeln!(xml, "\t\t\t\t</reason>").ok();
    }
}

impl Identifier {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t\t<identifiers>").ok();
        if let Some(envelope_to) = &self.envelope_to {
            writeln!(
                xml,
                "\t\t\t<envelope_to>{}</envelope_to>",
                escape_xml(envelope_to)
            )
            .ok();
        }
        writeln!(
            xml,
            "\t\t\t<envelope_from>{}</envelope_from>",
            escape_xml(&self.envelope_from)
        )
        .ok();
        writeln!(
            xml,
            "\t\t\t<header_from>{}</header_from>",
            escape_xml(&self.header_from)
        )
        .ok();
        writeln!(xml, "\t\t</identifiers>").ok();
    }
}

impl AuthResult {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t\t<auth_results>").ok();
        for dkim in &self.dkim {
            dkim.as_xml(xml);
        }
        for spf in &self.spf {
            spf.as_xml(xml);
        }
        writeln!(xml, "\t\t</auth_results>").ok();
    }
}

impl DKIMAuthResult {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t\t\t<dkim>").ok();
        writeln!(xml, "\t\t\t\t<domain>{}</domain>", escape_xml(&self.domain)).ok();
        writeln!(
            xml,
            "\t\t\t\t<selector>{}</selector>",
            escape_xml(&self.selector)
        )
        .ok();
        writeln!(xml, "\t\t\t\t<result>{}</result>", self.result).ok();
        if let Some(result) = &self.human_result {
            writeln!(
                xml,
                "\t\t\t\t<human_result>{}</human_result>",
                escape_xml(result)
            )
            .ok();
        }
        writeln!(xml, "\t\t\t</dkim>").ok();
    }
}

impl SPFAuthResult {
    pub(crate) fn as_xml(&self, xml: &mut String) {
        writeln!(xml, "\t\t\t<spf>").ok();
        writeln!(xml, "\t\t\t\t<domain>{}</domain>", escape_xml(&self.domain)).ok();
        writeln!(xml, "\t\t\t\t<scope>{}</scope>", self.scope).ok();
        writeln!(xml, "\t\t\t\t<result>{}</result>", self.result).ok();
        if let Some(result) = &self.human_result {
            writeln!(
                xml,
                "\t\t\t\t<human_result>{}</human_result>",
                escape_xml(result)
            )
            .ok();
        }
        writeln!(xml, "\t\t\t</spf>").ok();
    }
}

impl Display for Alignment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Alignment::Strict => "s",
            _ => "r",
        })
    }
}

impl Display for Disposition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Disposition::None | Disposition::Unspecified => "none",
            Disposition::Quarantine => "quarantine",
            Disposition::Reject => "reject",
        })
    }
}

impl Display for ActionDisposition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ActionDisposition::None | ActionDisposition::Unspecified => "none",
            ActionDisposition::Pass => "pass",
            ActionDisposition::Quarantine => "quarantine",
            ActionDisposition::Reject => "reject",
        })
    }
}

impl Display for DMARCResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DMARCResult::Pass => "pass",
            DMARCResult::Fail => "fail",
            DMARCResult::Unspecified => "",
        })
    }
}

impl Display for PolicyOverride {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PolicyOverride::Forwarded => "forwarded",
            PolicyOverride::SampledOut => "sampled_out",
            PolicyOverride::TrustedForwarder => "trusted_forwarder",
            PolicyOverride::MailingList => "mailing_list",
            PolicyOverride::LocalPolicy => "local_policy",
            PolicyOverride::Other => "other",
        })
    }
}

impl Display for DKIMResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DKIMResult::None => "none",
            DKIMResult::Pass => "pass",
            DKIMResult::Fail => "fail",
            DKIMResult::Policy => "policy",
            DKIMResult::Neutral => "neutral",
            DKIMResult::TempError => "temperror",
            DKIMResult::PermError => "permerror",
        })
    }
}

impl Display for SPFDomainScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SPFDomainScope::Helo => "helo",
            SPFDomainScope::MailFrom | SPFDomainScope::Unspecified => "mfrom",
        })
    }
}

impl Display for SPFResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SPFResult::None => "none",
            SPFResult::Neutral => "neutral",
            SPFResult::Pass => "pass",
            SPFResult::Fail => "fail",
            SPFResult::SoftFail => "softfail",
            SPFResult::TempError => "temperror",
            SPFResult::PermError => "permerror",
        })
    }
}

fn escape_xml(text: &str) -> Cow<'_, str> {
    for ch in text.as_bytes() {
        if [b'"', b'\'', b'<', b'>', b'&'].contains(ch) {
            let mut escaped = String::with_capacity(text.len());
            for ch in text.chars() {
                match ch {
                    '"' => {
                        escaped.push_str("&quot;");
                    }
                    '\'' => {
                        escaped.push_str("&apos;");
                    }
                    '<' => {
                        escaped.push_str("&lt;");
                    }
                    '>' => {
                        escaped.push_str("&gt;");
                    }
                    '&' => {
                        escaped.push_str("&amp;");
                    }
                    _ => {
                        escaped.push(ch);
                    }
                }
            }

            return escaped.into();
        }
    }
    text.into()
}

#[cfg(test)]
mod test {
    use crate::report::{
        ActionDisposition, Alignment, DKIMAuthResult, DKIMResult, DMARCResult, Disposition,
        Feedback, PolicyOverride, PolicyOverrideReason, Record, SPFAuthResult, SPFDomainScope,
        SPFResult,
    };

    #[test]
    fn dmarc_aggregate_report_generate() {
        let feedback = Feedback::new()
            .with_version(2.0)
            .with_org_name("Initech Industries Incorporated")
            .with_email("dmarc@initech.net")
            .with_extra_contact_info("XMPP:dmarc@initech.net")
            .with_report_id("abc-123")
            .with_date_range_begin(12345)
            .with_date_range_end(12346)
            .with_error("Did not include TPS report cover.")
            .with_domain("example.org")
            .with_version_published(1.0)
            .with_adkim(Alignment::Relaxed)
            .with_aspf(Alignment::Strict)
            .with_p(Disposition::Quarantine)
            .with_sp(Disposition::Reject)
            .with_testing(true)
            .with_record(
                Record::new()
                    .with_source_ip("192.168.1.2".parse().unwrap())
                    .with_count(3)
                    .with_action_disposition(ActionDisposition::Pass)
                    .with_dmarc_dkim_result(DMARCResult::Pass)
                    .with_dmarc_spf_result(DMARCResult::Fail)
                    .with_policy_override_reason(
                        PolicyOverrideReason::new(PolicyOverride::Forwarded)
                            .with_comment("it was forwarded"),
                    )
                    .with_policy_override_reason(
                        PolicyOverrideReason::new(PolicyOverride::MailingList)
                            .with_comment("sent from mailing list"),
                    )
                    .with_envelope_from("hello@example.org")
                    .with_envelope_to("other@example.org")
                    .with_header_from("bye@example.org")
                    .with_dkim_auth_result(
                        DKIMAuthResult::new()
                            .with_domain("test.org")
                            .with_selector("my-selector")
                            .with_result(DKIMResult::PermError)
                            .with_human_result("failed to parse record"),
                    )
                    .with_spf_auth_result(
                        SPFAuthResult::new()
                            .with_domain("test.org")
                            .with_scope(SPFDomainScope::Helo)
                            .with_result(SPFResult::SoftFail)
                            .with_human_result("dns timed out"),
                    ),
            )
            .with_record(
                Record::new()
                    .with_source_ip("a:b:c::e:f".parse().unwrap())
                    .with_count(99)
                    .with_action_disposition(ActionDisposition::Reject)
                    .with_dmarc_dkim_result(DMARCResult::Fail)
                    .with_dmarc_spf_result(DMARCResult::Pass)
                    .with_policy_override_reason(
                        PolicyOverrideReason::new(PolicyOverride::LocalPolicy)
                            .with_comment("on the white list"),
                    )
                    .with_policy_override_reason(
                        PolicyOverrideReason::new(PolicyOverride::SampledOut)
                            .with_comment("it was sampled out"),
                    )
                    .with_envelope_from("hello2example.org")
                    .with_envelope_to("other2@example.org")
                    .with_header_from("bye2@example.org")
                    .with_dkim_auth_result(
                        DKIMAuthResult::new()
                            .with_domain("test2.org")
                            .with_selector("my-other-selector")
                            .with_result(DKIMResult::Neutral)
                            .with_human_result("something went wrong"),
                    )
                    .with_spf_auth_result(
                        SPFAuthResult::new()
                            .with_domain("test.org")
                            .with_scope(SPFDomainScope::MailFrom)
                            .with_result(SPFResult::None)
                            .with_human_result("no policy found"),
                    ),
            );

        let message = feedback
            .as_rfc5322(
                "initech.net",
                "Initech Industries",
                "noreply-dmarc@initech.net",
                "dmarc-reports@example.org",
            )
            .unwrap();
        let parsed_feedback = Feedback::parse_rfc5322(message.as_bytes()).unwrap();

        assert_eq!(feedback, parsed_feedback);
    }
}
