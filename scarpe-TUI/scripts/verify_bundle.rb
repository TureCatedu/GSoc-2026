#!/usr/bin/env ruby
# frozen_string_literal: true

require "base64"
require "digest"

bundle_path = ARGV.fetch(0, "scarpe")
library_path = ARGV.fetch(1, "rust_core/target/release/librust_core.dylib")

bundle = File.binread(bundle_path)
library = File.binread(library_path)

marker = "\n__END__\n"
marker_index = bundle.index(marker)
abort "DATA section marker (__END__) not found" unless marker_index

payload = bundle[(marker_index + marker.bytesize)..]
decoded = Base64.decode64(payload)

expected_size = library.bytesize
actual_size = decoded.bytesize
abort "embedded payload size mismatch: expected #{expected_size}, got #{actual_size}" unless actual_size == expected_size

expected_sha = Digest::SHA256.hexdigest(library)
actual_sha = Digest::SHA256.hexdigest(decoded)
abort "embedded payload checksum mismatch: expected #{expected_sha}, got #{actual_sha}" unless actual_sha == expected_sha

puts "Bundle payload: ok"
puts "  bundle:  #{bundle_path}"
puts "  library: #{library_path}"
puts "  bytes:   #{actual_size}"
puts "  sha256:  #{actual_sha}"