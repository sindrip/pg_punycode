-- IDNA / UTS-46 domain conversions (WHATWG URL rules)
SELECT domain_to_ascii('Bücher.DE') AS mapped_and_encoded;
SELECT domain_to_ascii('中国') AS tld_zhongguo;
SELECT domain_to_ascii('рф') AS tld_rf;
SELECT domain_to_ascii('straße.de') AS non_transitional_eszett;
SELECT domain_to_ascii('💩.la') AS emoji;
SELECT domain_to_ascii('example.com') AS ascii_passthrough;
SELECT domain_to_unicode('xn--fiqs8s') AS u_label;
SELECT domain_to_unicode('XN--FIQS8S') AS case_insensitive_ace;
SELECT domain_to_ascii(domain_to_unicode('xn--mgbaam7a8h')) AS round_trip;

-- forbidden ASCII (URL rules) raises an error
SELECT domain_to_ascii('a b.com');
