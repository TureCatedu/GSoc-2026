require_relative '../lib/scarpe_tui'

Scarpe.app(title: "Scarpe-TUI Showcase Completo") do
  
  stack do
    para "=== BENVENUTO NELLA VETRINA DI SCARPE-TUI ==="
    para "Questa applicazione racchiude tutto ciò che abbiamo costruito."
    para "Usa il mouse o il tasto TAB per navigare."
    para "------------------------------------------------------------"
  end

  para "Come ti chiami?"
  nome_input = edit_line("")

  para "Qual è il tuo linguaggio di programmazione preferito?"
  lang_input = edit_line("")

  para "------------------------------------------------------------"

  flow do
    button "Genera Profilo" do

      nome = nome_input.text.strip
      lang = lang_input.text.strip

      if nome.empty? || lang.empty?
        para "--> ERRORE: Compila tutti i campi prima di generare!"
      else
        para " "
        para "*** NUOVO PROFILO GENERATO ***"
        para "Nome Utente: #{nome}"
        para "Linguaggio  : #{lang}"
        para "Livello     : Architetto di Interfacce Native"
        para "******************************"
      end
    end

    button "Spamma Testo (Test Scroll)" do

      para "Hai cliccato lo spam! Aggiungo questa riga per farti scrollare..."

    end

    button "Esci" do
      quit
    end
  end
end