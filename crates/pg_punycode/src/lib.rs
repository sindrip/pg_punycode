use pgrx::prelude::*;

::pgrx::pg_module_magic!(name, version);

/// Encode a string as Punycode (RFC 3492).
///
/// Label-level Punycode, not IDNA: no `xn--` prefix, no dot-splitting.
/// Pure-ASCII input gets a trailing `-` per the RFC. For domain names,
/// use `domain_to_ascii`/`domain_to_unicode`.
#[pg_extern(immutable, parallel_safe)]
fn punycode_encode(input: &str) -> String {
    idna::punycode::encode_str(input)
        .unwrap_or_else(|| error!("punycode_encode: invalid input: {input:?}"))
}

/// Decode a Punycode (RFC 3492) string. Raises an error on invalid input.
#[pg_extern(immutable, parallel_safe)]
fn punycode_decode(input: &str) -> String {
    idna::punycode::decode_to_string(input)
        .unwrap_or_else(|| error!("punycode_decode: invalid punycode: {input:?}"))
}

/// Convert a domain name to its IDNA ASCII ("xn--"/A-label) form, per
/// UTS-46 with WHATWG URL rules: case-mapped, normalized, validated.
#[pg_extern(immutable, parallel_safe)]
fn domain_to_ascii(input: &str) -> String {
    idna::domain_to_ascii_cow(input.as_bytes(), idna::uts46::AsciiDenyList::URL).map_or_else(
        |_| error!("domain_to_ascii: invalid domain name: {input:?}"),
        std::borrow::Cow::into_owned,
    )
}

/// Convert an IDNA ASCII (A-label) domain name back to Unicode, per
/// UTS-46 with WHATWG URL rules. Raises an error on invalid input.
#[pg_extern(immutable, parallel_safe)]
fn domain_to_unicode(input: &str) -> String {
    let (out, validity) = idna::uts46::Uts46::new().to_unicode(
        input.as_bytes(),
        idna::uts46::AsciiDenyList::URL,
        idna::uts46::Hyphens::Allow,
    );
    if validity.is_err() {
        error!("domain_to_unicode: invalid domain name: {input:?}");
    }
    out.into_owned()
}

#[cfg(any(test, feature = "pg_test"))]
// clippy's allow-unwrap-in-tests doesn't recognize this cfg form as test code
#[allow(clippy::unwrap_used)]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn encodes_via_sql() {
        let coded = Spi::get_one::<String>("SELECT punycode_encode('bücher')").unwrap();
        assert_eq!(coded, Some("bcher-kva".to_string()));
    }

    #[pg_test]
    fn decodes_via_sql() {
        let plain =
            Spi::get_one::<String>("SELECT punycode_decode('MajiKoi5-783gue6qz075azm5e')").unwrap();
        assert_eq!(plain, Some("MajiでKoiする5秒前".to_string()));
    }

    #[pg_test]
    fn round_trips_via_sql() {
        let plain =
            Spi::get_one::<String>("SELECT punycode_decode(punycode_encode('日本語'))").unwrap();
        assert_eq!(plain, Some("日本語".to_string()));
    }

    #[pg_test(error = "punycode_decode: invalid punycode: \"bücher\"")]
    fn decode_rejects_non_ascii() {
        Spi::get_one::<String>("SELECT punycode_decode('bücher')").unwrap();
    }

    #[pg_test]
    fn domain_to_ascii_via_sql() {
        let mapped = Spi::get_one::<String>("SELECT domain_to_ascii('Bücher.DE')").unwrap();
        assert_eq!(mapped, Some("xn--bcher-kva.de".to_string()));
        let tld = Spi::get_one::<String>("SELECT domain_to_ascii('中国')").unwrap();
        assert_eq!(tld, Some("xn--fiqs8s".to_string()));
        let ascii = Spi::get_one::<String>("SELECT domain_to_ascii('example.com')").unwrap();
        assert_eq!(ascii, Some("example.com".to_string()));
    }

    #[pg_test]
    fn domain_to_unicode_via_sql() {
        let tld = Spi::get_one::<String>("SELECT domain_to_unicode('XN--FIQS8S')").unwrap();
        assert_eq!(tld, Some("中国".to_string()));
        let round =
            Spi::get_one::<String>("SELECT domain_to_ascii(domain_to_unicode('xn--mgbaam7a8h'))")
                .unwrap();
        assert_eq!(round, Some("xn--mgbaam7a8h".to_string()));
    }

    #[pg_test(error = "domain_to_ascii: invalid domain name: \"a b.com\"")]
    fn domain_to_ascii_rejects_forbidden_ascii() {
        Spi::get_one::<String>("SELECT domain_to_ascii('a b.com')").unwrap();
    }
}

#[cfg(feature = "pg_bench")]
#[pg_schema]
mod benches {
    use pgrx::prelude::*;
    use pgrx_bench::{Bencher, black_box};

    #[pg_bench]
    fn bench_punycode_encode(b: &mut Bencher) {
        b.iter(|| {
            black_box(crate::punycode_encode(black_box("MajiでKoiする5秒前")));
        });
    }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
