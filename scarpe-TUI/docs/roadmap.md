# Scarpe TUI Roadmap

## Completato

- Backend Rust collegato alla DSL Ruby tramite FFI.
- Buffer terminale a doppio livello e rendering incrementale.
- Layout ricorsivo per root, stack, flow, border, dock bottom e scroll area.
- Input da tastiera e mouse.
- Editor Unicode per `EditLine` e `EditBox`.
- Movimento del cursore tra righe e mantenimento della colonna verticale.
- Inserimento di newline nell'`EditBox` con `Shift+Enter`.
- Fallback per terminali che espongono la combinazione come `Alt+Enter` o `Ctrl+Enter`.
- Invio con `Enter` normale.
- Test Rust per editor e buffer.
- Build debug e release verificate.
- Sintassi Ruby e controllo del diff verificati.

## Fase corrente

Consolidamento del percorso di distribuzione:

1. compilare il backend Rust in release;
2. rigenerare l'eseguibile bundled;
3. verificare che il bundle contenga la libreria release aggiornata;
4. mantenere separati gli artefatti di build dalle modifiche sorgenti.

## Attività futura: scrollbar visibile

La `ScrollArea` supporta già il calcolo dello spazio e lo scrolling, ma la scrollbar visuale non è ancora affidabile nel rendering reale. È registrata come attività futura e non blocca il lavoro corrente.

L'implementazione futura dovrà coprire:

- calcolo della proporzione tra contenuto e viewport;
- rendering della track e del thumb;
- aggiornamento del thumb durante tastiera e mouse;
- clipping corretto dei discendenti annidati;
- test di layout e rendering con contenuto corto, lungo e annidato.

## Prossime fasi

### Fase 1 — Distribuzione
- rigenerare il bundle release;
- verificare il caricamento FFI da sorgente e da bundle;
- aggiungere una verifica automatica non interattiva del bundle.

### Fase 2 — Robustezza API
- aggiungere test Ruby per la validazione dei codici di errore FFI;
- verificare aggiornamento dinamico del testo e callback di submit;
- controllare gestione shutdown e terminale.

### Fase 3 — UX TUI
- completare scrollbar visuale;
- migliorare focus e feedback visivo;
- aggiungere test per resize e mouse scrolling.

### Fase 4 — Qualità e distribuzione
- documentare compatibilità macOS/Linux;
- automatizzare test e build;
- ridurre le modifiche non pertinenti prima del commit.