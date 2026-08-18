require "minitest/autorun"

ROOT = File.expand_path("..", __dir__)

class AppSecurityContractTest < Minitest::Test
  def setup
    @source = File.read(File.join(ROOT, "examples", "app.rb"))
  end

  def test_api_key_is_not_exported_to_child_processes
    refute_match(/ENV\[['"]API_KEY['"]\]/, @source)
    assert_match(/api_key\s*=\s*config\.fetch\(["']api_key["']\)/, @source)
    assert_match(/unsetenv_others:\s*true/, @source)
  end

  def test_ai_file_writes_are_workspace_validated
    assert_equal 2, @source.scan(/safe_workspace_path\(workspace_root, path\)/).length
    assert_equal 2, @source.scan(/write_private_file\(safe_path, content\.strip\)/).length
    refute_match(/File\.write\(path,\s*content\.strip\)/, @source)
  end

  def test_stream_parser_keeps_partial_lines
    assert_match(/sse_buffer\s*<<\s*chunk/, @source)
    assert_match(/sse_buffer\s*=\s*complete_lines\.pop\.to_s/, @source)
    assert_match(/complete_lines\.each do \|line\|/, @source)
  end

  def test_sensitive_files_are_written_private
    assert_match(/File\.chmod\(0o600,\s*config_file\)/, @source)
    assert_operator @source.scan(/write_private_file\(history_file/).length, :>=, 1
  end
end