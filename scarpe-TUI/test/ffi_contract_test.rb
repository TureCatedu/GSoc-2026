require "minitest/autorun"

ROOT = File.expand_path("..", __dir__)

class FfiContractTest < Minitest::Test
  def test_free_string_uses_pointer_abi
    ffi_source = File.read(File.join(ROOT, "ext", "mylib.rb"))
    ruby_source = File.read(File.join(ROOT, "lib", "scarpe_tui.rb"))
    rust_source = File.read(File.join(ROOT, "rust_core", "src", "ffi.rs"))

    assert_match(
      /attach_function\s+:scarpe_tui_free_string,\s*\[:pointer\],\s*:void/,
      ffi_source
    )
    assert_match(/scarpe_tui_free_string\(str_ptr\)/, ruby_source)
    refute_match(/scarpe_tui_free_string\(str_ptr\.address\)/, ruby_source)
    assert_match(
      /fn scarpe_tui_free_string\(s:\s*\*mut c_char\)/,
      rust_source
    )
  end

  def test_manifest_uses_cargo_filename
    manifest_names = Dir.children(File.join(ROOT, "rust_core"))

    assert_includes manifest_names, "Cargo.toml"
    refute_includes manifest_names, "cargo.toml"
  end
end
