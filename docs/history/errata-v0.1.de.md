# Base65t v0.1 — Errata

> **Historie.** Der aktuelle Stand ist `docs/spec-v0.4.md`. Dieses Dokument
> ist nicht normativ; es liegt hier, weil es trägt, wie es dazu kam.
> `docs/history/README.md` sagt, was zwischen den Fassungen gestrichen wurde
> und warum.

**Eingearbeitet in `spec-v0.2.de.md`.** Dieses Dokument bleibt liegen, weil es
die Begründungen und die Messungen trägt, die v0.2 nur noch als Ergebnis
ausspricht.

`spec-v0.1.de.md` ist final und bleibt Wort für Wort, wie es ist. Dieses
Dokument sagt, was daran nicht trägt, und was stattdessen gilt.

Grundlage: `FINDINGS.md` (was die Referenzimplementierung gefunden hat),
`PREREGISTRATION.md` (die Messregel, festgelegt vor der Messung) und der
Messanhang unten.

**Wie entschieden wurde.** Nicht jede offene Stelle ist eine Datenfrage. Die
Spec ordnet ihre drei Achsen selbst: Größe ist in §9.4 **normativ**, Durchsatz
ist in §2 ein ausdrückliches **Nicht-Ziel**, Lesbarkeit ist Ziel 3. Damit steht
die Form der Antwort vor jeder Messung fest — *maximiere die Lesbarkeit, solange
die Ausgabe nie länger als Base64 wird* — und zu messen blieb nur, wie viel
Lesbarkeit dieser Rand hergibt. Sechs der zehn Punkte unten brauchten gar keine
Messung.

---

## E1 — §11.1, *Berechnung*: die Ordnung ist die Definition

**Alt:**

> … indem er an jeder Position unter den längenoptimalen Fortsetzungen zuerst
> `B` wählt, sonst das **längste** zulässige Literal. O(n), und das Ergebnis
> ist per Konstruktion das Minimum von `Key`.

**Neu:**

> … indem er an jeder Position das **kleinste Symbol** wählt, das noch eine
> längenoptimale Fortsetzung zulässt: `B`, wo ein Base64-Segment optimal
> beginnen kann; sonst `L`, solange das Literal optimal weiterlaufen kann;
> sonst `S`. Das ist genau die Ordnung, und deshalb ihr Minimum.

**Warum.** Die alte Fassung berechnet nicht, was die Ordnung definiert. Ein
Literal früh zu beenden richtet den Base64-Lauf dahinter so aus, dass später
noch ein Literal längenoptimal wird; `B < L` entscheidet dann für das kürzere.
Kleinstes Gegenbeispiel, zehn Bytes: `"aaaaaaaaa "` ergibt `~HaaaaaaaYWEg`
nach der Ordnung und `~JaaaaaaaaaIA` nach der Berechnung, beide 13 Zeichen.

Die **Ordnung** bleibt, weil sie die Definition ist und die Berechnung nur
beansprucht, sie auszurechnen. Der Satz über `B < L, S` und der über `L < S`
bleiben unverändert gültig; der zweite ist als Aussage über *angrenzende
Literale* zu lesen, nicht als Zusicherung maximaler Literale.

**Nachweis.** `neither_rule_dominates_on_passthrough`,
`divergence_from_the_berechnung_paragraph`, `ten_bytes_is_the_shortest_disagreement`
und `tests/canonical.rs`, das `encode_canonical` gegen die erschöpfende
Aufzählung aller gültigen Segmentierungen prüft.

## E2 — §11.1, *Berechnung*: „O(n)" gilt der Kostentabelle

**Alt:** „O(n), und das Ergebnis ist per Konstruktion das Minimum von `Key`."

**Neu:** „Der Rückwärtslauf ist O(n) nach §9.2. Für den Vorwärtslauf ist keine
O(n)-Schranke bewiesen: die Deques liefern das Minimum, die Rekonstruktion
braucht das Argument des Minimums unter einer Tie-Break-Regel, und das ist eine
andere Anfrage. Die Referenzimplementierung zählt die zulässigen Enden je
Literal auf, was O(Fenster) je Literal kostet."

## E3 — §9.0: eine Tie-Break-Regel für das ganze Format

**Neu, als Absatz an §9.0 anzufügen:**

> Wo mehrere gültige Segmentierungen dieselbe Länge haben, MUSS ein Encoder die
> nach der Ordnung aus §11.1 kleinste wählen. Ausgenommen ist `legible`, das
> nach E4 eine eigene Zielfunktion hat.

**Warum.** Ohne diese Regel ist kein Testvektor byte-exakt: für TV2 sind drei
Segmentierungen 26 Zeichen lang, weil das Aufnehmen von ein oder zwei
Textbytes in das Base64-Segment nichts kostet. §16.8 will den Vektorsatz auf
200 ausbauen — mit drei erlaubten Antworten je Vektor ginge das nicht.

Gemessen kostet die Regel bei `dense` nichts: bei `L_min = 11` liegen alle
Kandidatenregeln innerhalb von 0,4 Punkten Klartextanteil und bei identischer
Segmentzahl (Anhang, Tabelle 3). Sie ist eine Festlegung, keine Abwägung.

**TV2 wird damit:**

```
TV2  DE AD BE EF "session-eu-central"   -> 3q2-73Nl~Qssion-eu-central   (26 vs. 30)
```

## E4 — §9.3 und §9.4: `legible` bekommt eine Zielfunktion

**Alt:** `legible` ist in §9.3 durch den Schwellwert `L ≥ 4` definiert, und §9.4
nimmt es von der Nie-schlechter-Garantie aus.

**Neu:**

> `legible` minimiert die Länge und wählt unter den längengleichen
> Segmentierungen die mit dem **größten Klartextanteil**. Es hat keinen
> Schwellwert. §9.4 gilt für `legible` **mit**; die Ausnahme entfällt.

**Warum.** Ein Schwellwert kann Ausgaben nicht lesbarer machen: die Zielfunktion
bleibt nach §9.0 die Länge, also wird ein Literal, das etwas kostet, nie
gewählt, und `legible` fällt mit `dense` zusammen. Das war der Befund; die
Frage war, wie viel Lesbarkeit sich kaufen lässt und zu welchem Preis.

Die Messung beantwortet sie deutlich (Anhang): **jeder** Bonus, der groß genug
ist, ein kostendes Literal zu kaufen, verletzt §9.4 — bei λ = ⅓ Zeichen je
Byte auf 35 von 87 Korpusdateien unter Profil U, auf 14 von 87 unter T und auf
2 von 12 Silesia-Dateien. Ziel 1 ist normativ und Lesbarkeit ist es nicht, also
fällt jeder Bonus. Der Gleichstand dagegen ist **umsonst**: dieselbe Länge,
dieselbe Garantie, und +5,4 Punkte Klartextanteil unter T (+6,7 unter U, +2,4
auf Silesia), auf **jeder** Datei und nicht nur im Mittel.

**Preis, offen genannt:** die Segmentzahl steigt. Unter T um 24 %, auf Silesia
um 6 %, über den Achsen-Sweep um 15 % — alles innerhalb der vorregistrierten
Grenze von 50 %. Unter **Profil U** auf dem Korpus sind es 67 % und damit
darüber. Das ist die eine Stelle, an der die Prä-Registrierung nicht
vollständig war: sie hat die Grenze nicht je Profil festgelegt. Aufgelöst wird
sie mit §0.1, wo `legible` mit Profil T und mit „Lesbarkeit **vor** Größe"
geführt wird — was vor der Größe steht, steht erst recht vor einem Nicht-Ziel
(§2). Wer `legible` unter Profil U einsetzt, zahlt rund 70 % mehr Segmente — auf dem
Korpus 1,67×, auf dem Hold-out 1,74×. Das ist gemessen und reproduziert sich,
es ist also keine Unsicherheit, sondern ein Preis: unter U ist `legible` eine
Entscheidung für Lesbarkeit gegen Durchsatz, unter T ist sie fast umsonst.

**TV5a wird damit:**

```
Body (legible)   : aGVsbG9-QWxpY2U   (15 chars) -> wie dense
```

Der bisherige Wert `~Fhellofg~FAlice` ist 16 Zeichen lang und damit unter jeder
längenminimierenden Zielfunktion unerreichbar. Er bleibt ein gültiger Strom und
dekodiert korrekt; er ist nur nichts, was ein Encoder schreibt.

## E5 — TV11: ein Fehlercode, nicht zwei

**Alt:** `"~Aabc" -> decode() : framed, dann E_TRUNCATED / E_FRAME_SYNC`

**Neu:** `"~Aabc" -> decode() : framed, dann E_TRUNCATED`

**Warum.** §10.3 prüft Marker, dann Länge. `abc` ist eine wohlgeformte
18-Bit-Länge von 108252, die der fünf Zeichen lange Strom nicht erfüllen kann.
`E_FRAME_SYNC` ist nicht erreichbar, und die Prüfreihenfolge in §10.3 ist
normativ.

## E6 — §5.3 und §5.4: „Strom" heißt immer der ganze Oktett-Strom

**Neu, als Absatz an §5.3 und §5.4 anzufügen:**

> Wo diese Spezifikation vom *Strom* spricht, ist der vollständige
> Oktett-Strom gemeint, nie ein Frame-Body. Regel P gilt deshalb **nur im
> Plain Mode**: ein `=` innerhalb eines Frames ist kein Padding, sondern nach
> §10.4 ein Zeichen an einer Alphabetposition, das keines ist
> (`E_CHARSET`). Regel A gilt über den gesamten Strom, also auch über
> Framegrenzen hinweg.

**Warum.** §8.1 nennt einen Frame-Body einen „Plain-Mode-Stream" und §10.3
reicht ihn an `decode_plain`, für das der Body der Strom ist. Damit zerfällt
die Spec in zwei Lesarten, und zwei Dekoder können über denselben Strom
verschieden urteilen, während beide ihr folgen — genau die Fläche, gegen die
§14 sonst argumentiert. Padding existiert, damit ein Erzeuger gewöhnlichen
Base64 nichts ändern muss (§1.1); kein solcher Erzeuger schreibt Frames. Innen
kostet es nur Angriffsfläche.

## E7 — §10.3: die Marker-Prüfung braucht eine Längenprüfung

**Neu, in die Fallenliste unter §10.1 aufzunehmen:**

> **(4)** Der Vergleich `stream[pos..pos+2] == "~A"` in §10.3 steht **vor**
> `pos + 5 <= len`. Die Reihenfolge ist richtig — ein nicht gerahmter Strom
> soll das sagen und nicht „abgeschnitten" —, aber der Vergleich selbst MUSS
> längensicher sein: bei `pos = len − 1` liest eine wörtliche Umsetzung über
> das Ende hinaus.

## E8 — §12: die Binärzeile ist eine Aussage über Profil U und T

**Alt:** `| Rein binär | 1,333 | **1,333** *(exakt)* | … |`

**Neu:** `| Rein binär (Profil U, T) | 1,333 | **1,333** *(exakt)* | … |`, mit
dem Zusatz:

> Unter Profil B ist jedes Byte literalfähig; der Encoder schreibt ein einziges
> Literalsegment, und die Dichte ist auch für Binärdaten 1,00096. Profil B ist
> kein Textencoding (§7) und die Tabelle beschreibt es nicht.

## E9 — §16.3 und §16.8: der Vektorsatz ist veröffentlicht

`docs/vectors.json` enthält 456 Vektoren über alle fünf Presets und alle drei
Profile, je als Eingabe und erwarteten Strom in Hex. Damit ist die
übertragbare Hälfte von Nachweis 3 erbracht: eine zweite Implementierung prüft
sich gegen diese Bytes, ohne eine Zeile der ersten zu lesen. Der Nachweis
selbst bleibt offen, bis es sie gibt — eine Implementierung kann ihn nicht
führen, und `tests/canonical.rs` (erschöpfende Aufzählung gegen den DP) ist der
nächstbeste Ersatz, nicht der Nachweis.

## E10 — §9.5 bleibt offen, und das ist jetzt begründet

`L_min` und `B_min` sind weiter unbestimmt. Die Entscheidungen oben nehmen
ihnen nichts vorweg: `canonical` hat kein `L_min` (§11.1), `legible` hat nach
E4 keines mehr, und `dense` behält die aus §9.1 **hergeleitete** 11. Eine
spätere Messung darf keine der drei ändern — sonst änderten Messergebnisse
rückwirkend Cache-Keys.

---

## Messanhang

Instrument: `cargo run --release --example sweetspot`. Alle Zahlen exakt und
deterministisch; kein Durchsatz, weil §9.5 ihn an die Segmentrate bindet und
die hier steht. Regelwerk und Schwellen: `PREREGISTRATION.md`, festgelegt vor
dem Lauf.

**Tabelle 1 — Entscheidungsdaten, 88 Samples (8,5 MB), Profil U, ohne Schwellwert**

| Regler | Klartext | Größe zu Base64 | schlechteste Datei | über Base64 | Segmente/kB |
|---|---|---|---|---|---|
| λ=0, Ordnung §11.1 | 12,59 % | 98,8 % | 100,0 % | 0 von 87 | 25,4 |
| λ=0, längstes Literal | 11,21 % | 98,8 % | 100,0 % | 0 von 87 | 20,8 |
| **λ=0, Klartext-Vorrang** | **19,26 %** | **98,8 %** | **100,0 %** | **0 von 87** | 42,4 |
| λ=1, Klartext-Vorrang | 31,27 % | 100,6 % | 107,9 % | **35 von 87** | 93,6 |
| λ=2 | 34,53 % | 102,0 % | 118,4 % | 44 von 87 | 115,7 |
| λ=3, λ=4 | 36,83 % | 103,7 % | 147,8 % | 46 von 87 | 140,3 |

**Tabelle 2 — dieselben Daten, Profil T**

| Regler | Klartext | Größe | schlechteste | über Base64 | Segmente/kB |
|---|---|---|---|---|---|
| λ=0, Ordnung §11.1 | 39,48 % | 93,1 % | 100,0 % | 0 von 87 | 38,2 |
| λ=0, längstes Literal | 38,47 % | 93,1 % | 100,0 % | 0 von 87 | 35,0 |
| **λ=0, Klartext-Vorrang** | **44,85 %** | **93,1 %** | **100,0 %** | **0 von 87** | 47,2 |
| λ=1, Klartext-Vorrang | 53,43 % | 94,4 % | 109,1 % | **14 von 87** | 82,9 |
| λ=2 | 56,00 % | 95,5 % | 109,1 % | 18 von 87 | 101,5 |
| λ=3, λ=4 | 59,25 % | 97,9 % | 127,3 % | 19 von 87 | 136,1 |

**Tabelle 3 — `dense` (Profil U, `L_min = 11`): der Tie-Break ist gleichgültig**

| Regler | Klartext | Größe | über Base64 | Segmente/kB |
|---|---|---|---|---|
| λ=0, Ordnung §11.1 | 6,09 % | 99,1 % | 0 von 87 | 8,5 |
| λ=0, längstes Literal | 6,16 % | 99,1 % | 0 von 87 | 8,5 |
| λ=0, Klartext-Vorrang | 6,48 % | 99,1 % | 0 von 87 | 8,5 |
| λ ≥ 1 | 6,48 % | 99,1 % | 0 von 87 | 8,5 |

Der Schwellwert aus §9.1 schluckt den Regler vollständig. Keine Regel gewinnt
nach der vorregistrierten Schranke (1 Punkt Klartext oder 5 % Segmente), also
erbt `dense` die Ordnung aus §11.1 — E3.

**Tabelle 4 — Hold-out: Silesia, 202 MiB.** Einmal angefasst, nach der Wahl.
Profil T:

| Regler | Klartext | Größe | schlechteste | über Base64 | Segmente/kB |
|---|---|---|---|---|---|
| λ=0, Ordnung §11.1 | 55,03 % | 88,6 % | 100,0 % | 0 von 12 | 28,1 |
| λ=0, längstes Literal | 54,65 % | 88,6 % | 100,0 % | 0 von 12 | 27,1 |
| **λ=0, Klartext-Vorrang** | **57,44 %** | **88,6 %** | **100,0 %** | **0 von 12** | 29,7 |
| λ=1, Klartext-Vorrang | 60,37 % | 89,0 % | 101,8 % | **2 von 12** | 39,4 |
| λ=2 | 61,61 % | 89,6 % | 105,2 % | 3 von 12 | 48,7 |
| λ=3 | 63,32 % | 90,8 % | 112,2 % | 4 von 12 | 67,3 |

Profil U:

| Regler | Klartext | Größe | schlechteste | über Base64 | Segmente/kB |
|---|---|---|---|---|---|
| λ=0, Ordnung §11.1 | 12,44 % | 98,5 % | 100,0 % | 0 von 12 | 21,6 |
| λ=0, längstes Literal | 11,46 % | 98,5 % | 100,0 % | 0 von 12 | 18,3 |
| **λ=0, Klartext-Vorrang** | **18,52 %** | **98,5 %** | **100,0 %** | **0 von 12** | 37,6 |
| λ=1, Klartext-Vorrang | 28,47 % | 100,0 % | 104,5 % | **7 von 12** | 81,8 |
| λ=2 | 32,57 % | 101,8 % | 109,6 % | 9 von 12 | 109,9 |
| λ=3, λ=4 | 36,57 % | 104,8 % | 119,4 % | 10 von 12 | 152,6 |

Die Grenze, die λ ≥ 1 verwirft, reproduziert sich auf Daten, die Fremde 2003
für einen fremden Zweck zusammengestellt haben — unter beiden Profilen, 2 von
12 Dateien unter T und 7 von 12 unter U. Das ist die Bestätigung, die Regel 5
verlangt.

Die Segmentgrenze reproduziert sich ebenfalls, und zwar auf der Seite, die
nicht ins Bild passt: unter Profil U liegt der Klartext-Vorrang auf dem
Hold-out bei 1,74× und damit ebenso über der vorregistrierten Grenze wie auf
dem Korpus (1,67×). Der Nachtrag unten steht also nicht wegen einer
Unsicherheit, sondern wegen eines Preises, der unter U real ist.

**Tabelle 5 — Achsen-Sweep**, 40 × 64 KiB, Anteil profilwidriger Bytes 0–100 %
über Lauflängen 4/16/64/256 — die Achse, die der Korpus nicht abfährt.

| Regler | Klartext | über Base64 | Segmente/kB |
|---|---|---|---|
| λ=0, Ordnung §11.1 | 68,47 % | 0 von 40 | 10,1 |
| **λ=0, Klartext-Vorrang** | **69,46 %** | **0 von 40** | 11,6 |
| λ=1 | 69,91 % | 0 von 40 | 13,5 |
| λ=3 | 70,04 % | 1 von 40 | 14,5 |

**Anwendung der Regel.** Regel 1 verwirft jedes λ ≥ 1: `S ≤ 100 %` je Datei ist
auf den Entscheidungsdaten verletzt (35 bzw. 14 von 87) und auf dem Hold-out
(2 von 12). Regel 4 macht daraus λ = 0. Unter den Tie-Break-Regeln gewinnt der
Klartext-Vorrang nach der dafür vorregistrierten Schranke — mehr als ein Punkt
Klartext, und das Vorzeichen auf **jeder** Datei, nicht nur im Mittel; das ist
hier kein Befund, sondern Konstruktion, weil die Zielfunktion lexikographisch
ist. Bei `dense` erreicht keine Regel die Schranke, also bleibt es bei §11.1.

**Nachtrag zur Prä-Registrierung.** Sie legt die Segmentgrenze `G(λ) ≤ 1,5·G(0)`
fest, ohne ein Profil zu nennen. Der Klartext-Vorrang hält sie unter T (1,24×),
auf Silesia (1,06×) und über den Achsen-Sweep (1,15×), nicht aber unter Profil
U — dort 1,67× auf dem Korpus und 1,74× auf dem Hold-out, also durchgängig.
Aufgelöst über §0.1, wo `legible` mit Profil T und
„Lesbarkeit vor Größe" geführt wird. Die Lücke wird hier benannt statt
nachträglich geschlossen.
