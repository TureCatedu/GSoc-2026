require_relative '../lib/scarpe_tui'

Scarpe.app(title: "Scarpe-TUI Full Showcase") do
  
  stack do
    para "=== WELCOME TO THE SCARPE-TUI SHOWCASE ==="
    para "This application encompasses everything we have built so far."
    para "Use the mouse or the TAB key to navigate."
    para "------------------------------------------------------------"
  end

  para "What is your name?"
  name_input = edit_line("")

  para "What is your favorite programming language?"
  language_input = edit_line("")

  para "------------------------------------------------------------"

  flow do
    button "Generate Profile" do

      name = name_input.text.strip
      language = language_input.text.strip

      if name.empty? || language.empty?
        para "--> ERROR: Please fill in all fields before generating!"
      else
        para " "
        para "*** NEW PROFILE GENERATED ***"
        para "Username : #{name}"
        para "Language : #{language}"
        para "Level    : Native Interface Architect"
        para "******************************"
      end
    end

    button "Spam Text (Scroll Test)" do
      para "You clicked spam! Adding this line to test the scrolling behavior..."
    end

    button "Quit" do
      quit
    end
  end
end
