require_relative '../lib/scarpe_tui'

Scarpe.app(true, title: "Test Finale Scarpe-TUI") do
  
  stack do
    para "=== GENERATORE DI PROFILI GSoC 2026 ===", stroke: "cyan", modifier: "bold"
    para "Compila i campi qui sotto con la tastiera", stroke: "blue"
    para "e usa il MOUSE per cliccare i bottoni."
    para "----------------------------------------"
  end

  para "Come ti chiami?"
  nome_input = edit_line("", stroke: "dark_yellow")

  para "Qual e' il tuo linguaggio preferito?"
  lang_input = edit_line("", stroke: "dark_yellow")

  para " "

  termini_checkbox = nil 
  
  flow do
    termini_checkbox = checkbox stroke: "magenta"
    para " Accetto i termini e condizioni"
  end

  para " "

  flow do
    button "Genera Profilo", stroke: "dark_green" do
      
      nome = nome_input.text.strip
      linguaggio = lang_input.text.strip
      
      accettato = termini_checkbox.checked?

      if nome.empty? || linguaggio.empty?
        para "-> ERRORE: Compila tutti i campi prima di generare!", stroke: "red"
      elsif !accettato
        para "-> ERRORE: Devi accettare i termini e condizioni per proseguire!", stroke: "red"
      else
        para " "
        para "*** NUOVO PROFILO GENERATO ***", stroke: "green", modifier: "bold"
        para "Nome Utente: #{nome}"
        para "Linguaggio : #{linguaggio}"
        para "Qualifica  : Architetto di Interfacce Native"
        para "Termini    : Accettati"
        para "******************************"
      end
    end

    button "Esci" do
      quit
    end
  end

end