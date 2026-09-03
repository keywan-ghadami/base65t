# Wie es dazu kam

Hier liegt nichts, wonach man implementieren soll. Der Stand steht in
`docs/spec-v0.4.de.md`; dieser Ordner beantwortet die andere Frage — **warum
steht es so da** — und er beantwortet sie, weil die interessanten
Entscheidungen alle gegen die naheliegende Variante ausgefallen sind.

Wer das Format benutzen will, braucht diesen Ordner nicht. Wer es
weiterentwickeln oder ein zweites Mal implementieren will, findet hier die
Begründungen und vor allem die Messungen, die einzelne Sätze der Spezifikation
erzwungen haben.

## Die Dokumente

| Datei | Was darin steht |
|---|---|
| `spec-v0.1.de.md` | Die erste Fassung. Fünf Presets, Framed Mode, drei Profile, ein Greedy-Encoder als zulässige Alternative. Vollständig überholt, aber jede spätere Abschnittsnummer stammt von hier |
| `errata-v0.1.de.md` | Zehn Entscheidungen (E1–E10), die beim Implementieren von v0.1 fällig wurden. E1 ist der Fund, dass §11.1 zwei einander widersprechende Definitionen der kanonischen Form enthielt |
| `spec-v0.2.de.md` | v0.1 + Errata, plus die lineare Regel und `dense-fast`. Der Zwischenstand, gegen den die Performance-Arbeit gemessen wurde |
| `spec-v0.4-segmente.de.md` | Das Segmentformat mit **einem** Encoder statt fünf, am Kopf entschieden, exakt programmiert, gefenstert. Trug die Nummer v0.4 einen Tag lang; §13.3 darin ist die Messung, die es gekippt hat |
| `FINDINGS.md` | Was das Implementieren gefunden hat: Widersprüche, zu enge Suchräume, Zahlen, die nicht stimmten. Chronologisch, nicht redigiert |
| `PREREGISTRATION.md` | Die Sweetspot-Messung, festgelegt **bevor** sie lief. Damit der Schwellwert `L_min = 11` nicht das Ergebnis einer nachträglich passend gewählten Auswertung ist |

## Was zwischen v0.2 und v0.4 passiert ist

Es gibt kein `spec-v0.3.de.md`. v0.3 war ein Stand im Code, kein Dokument: die
lineare Regel, die Parallelisierung, das Preset `dense-fast`, die
vektorisierte Base64-Schleife. Was daran überlebt hat, steht in v0.4; was nicht
überlebt hat, steht in der Commit-Historie und hier zusammengefasst, weil vier
gestrichene Ideen mehr über das Format sagen als die vier, die geblieben sind.

**Gestrichen: `legible`.** Ein Preset, das bei Längengleichstand die lesbarere
Segmentierung wählt. Der Tie-Break brauchte eine zweite Kostenkomponente, und
deren lexikografischer Vergleich kostete das Programm aus §9.2 zwischen 60 und
190 % — bei *jedem* Preset, auch den vieren, die ihn nie verlangt haben. Ein
Feature, das nur einer wollte, hat allen die Rechnung geschickt.

**Gestrichen: der Framed Mode.** Er war die eine Stelle, die §9.4 nicht abdecken
konnte, und eine Garantie mit Ausnahme ist eine Garantie, die niemand zitiert.
Der Preis wären fünf Zeichen je 64 KiB gewesen; der Gegenwert war ein
Zufallszugriff, den niemand angefragt hatte. `~A` bleibt reserviert, damit eine
spätere Revision die Frage neu stellen kann.

**Gestrichen: Profil B.** Ein Profil, in dem ein Literal jedes Oktett tragen
darf. Damit ist die Ausgabe kein Text mehr, und „die Ausgabe ist Text" ist der
Satz, wegen dem jemand das Format überhaupt anschaut. Ein Profil, das den
Kernsatz mit einer Fußnote versieht, kostet mehr als es bringt.

**Gestrichen: die Presets selbst.** Fünf, dann sechs, dann null. Der Grund
steht in §0.1 der v0.4: wer zwischen einem dichten und einem schnellen Encoder
wählen muss, muss erst wissen, was diese Wörter bedeuten, und wer unsicher ist,
greift zu Base64. Die Wahl trifft jetzt §9.6 anhand des Dateikopfs.

**Gestrichen: die lineare Regel.** Sie war die Antwort auf „der exakte DP ist
zu langsam für große Daten" und hat 0,22 % Dichte gekostet. Die bessere Antwort
war, bei großen Daten *gar nicht erst hinzuschauen*, wenn der Kopf sagt, dass
nichts zu finden ist — und den exakten DP zu behalten, wo etwas zu finden ist.
Der Faktor, um den der DP langsamer ist, wurde dabei zweimal falsch berichtet
(erst „12×", dann gemessen 21–63×); die Korrektur steht in `FINDINGS.md`.

## Und dann das Segmentformat selbst

Mit einem Encoder statt fünf war das Segmentformat konsequent, und es war
gemessen: auf durchgehend legalen kurzen Werten schneller als Base64, auf
komprimierten Daten byteweise Base64. Dazwischen, auf gemischtem Text, kostete
das exakte Programm das Sechs- bis Elffache der Base64-Zeit für null bis
anderthalb Prozent Größe — und jeder Versuch, das zu beheben, war ein weiterer
Mechanismus: eine Kopfentscheidung, eine Fensterung, eine geschlossene Form,
eine Stichprobe. Fünf Mechanismen, um eine Idee bezahlbar zu machen.

Die Frage, die das beendet hat, war nicht "wie machen wir das Programm
schneller", sondern "warum müssen wir überhaupt nachschauen". Ein Block fester
Länge, drei Formen, ein Bit je Byte für die Maske, das häufigste Muster mit
einem eigenen kurzen Code. Kein Suchen, kein Zustand, keine Längen im Strom.
Das ist `docs/spec-v0.4.de.md`, und die Segmentfassung liegt daneben als das,
was ein Tag Messen kostet und lehrt.

**Geblieben aus der Segmentzeit:** die Maske über 64 Bytes (sie ist jetzt das
ganze Encoder-Innere), die Regel-A-Erkenntnis, dass Alphabet-Konsistenz eine
Suche und keine Dekodierung ist, und die Base64-Schleife, die ein Segment in
ein Slice bekannter Länge schreibt.
