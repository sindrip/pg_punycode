-- RFC 3492 sample vectors through SQL
SELECT punycode_encode('bücher') AS encoded;
SELECT punycode_encode('London') AS ascii_gets_delimiter;
SELECT punycode_decode('MajiKoi5-783gue6qz075azm5e') AS decoded;
SELECT punycode_decode(punycode_encode('日本語')) = '日本語' AS round_trip;

-- function flags: strict, immutable, parallel safe
SELECT proisstrict, provolatile, proparallel
FROM pg_proc
WHERE proname IN ('punycode_encode', 'punycode_decode')
ORDER BY proname;

-- invalid input raises an error
SELECT punycode_decode('bücher');
