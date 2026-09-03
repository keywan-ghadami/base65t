# Base65t — Spezifikation v0.4, Maskenfassung (zurückgezogen)

> **Historie.** Diese Fassung trug die Nummer v0.4 für einen Tag. Sie ist das
> Blockformat mit einer dritten Blockform: einer Maske mit einem Bit je Byte,
> die die zulässigen Bytes eines gemischten Blocks im Klartext ließ. Sie ist
> die lesbarste Fassung, die es je gab, und die einzige, die dafür einen
> messbaren Preis verlangte — §13.1 darin ist die Messung. Der aktuelle Stand
> ist `docs/spec-v0.4.de.md`; `docs/history/README.md` sagt, warum.


**Status:** aktuell. **Wire-Format: nicht stabil.** v0.4 ersetzt das
Segmentformat der Fassungen v0.1 bis v0.3 durch Blöcke fester Länge, und
nichts verspricht, dass v0.5 die Blöcke behält. Was stabil ist, ist der
Vertrag: Bytes hinein, druckbares ASCII heraus, nie länger als Base64, und
jeder Base64-Strom wird gelesen. Wer heute Ströme speichert, speichert die
Versionsnummer daneben.

Die früheren Fassungen liegen in `docs/history/`, zusammen mit einem
Verzeichnis dessen, was zwischen den Fassungen gestrichen wurde und warum.

**Kurzfassung:** Base64URL, erweitert um ein 65. Zeichen (`~`). Die Eingabe
wird in Blöcke von 48 Bytes geschnitten; jeder Block ist entweder Base64,
oder er steht roh da, oder er trägt eine Maske, die sagt, welche seiner Bytes
roh dastehen und welche als Base64 folgen.

> Normative Aussagen sind als solche gekennzeichnet und verwenden MUSS / DARF NICHT /
> SOLLTE nach RFC 2119. Zahlen, die nicht als *exakt* markiert sind, sind Messungen
> auf dem in §16.5 genannten Korpus.

## Änderungen gegenüber der Segmentfassung

Die Abschnittsnummern sind beibehalten, wo der Gegenstand derselbe ist, damit
die Verweise aus `docs/history/` weiter tragen. Wo ein Abschnitt etwas anderes
beschreibt als vorher, steht das am Anfang des Abschnitts.

| § | Änderung |
|---|---|
| 4 | **Blöcke statt Segmente.** Drei Blockformen fester Länge; keine Längen im Strom |
| 6 | War das Literal-Segment mit Längen-Header; ist jetzt der Maskenblock |
| 9 | Der Encoder ist eine Abbildung je Block, ohne Suche und ohne Zustand. §9.2 (das Programm), §9.2.1 (Fensterung), §9.2.4 (geschlossene Form), §9.6 (Kopfentscheidung) entfallen |
| 10 | Der Dekoder kennt vor dem Lesen eines Blocks dessen Länge |
| 10.4 | `E_RESERVED_LEN` entfällt, `E_MASK` kommt hinzu |
| 11 | Kanonizität folgt aus der Abbildung; die Ordnung `B < L < S` entfällt |
| 13 | Neu gemessen. Kodieren und Dekodieren liegen in beiden Profilen bei Base64 |
| 14 | Der Dekoder parst keine angreifergewählte Länge mehr |
| 15 | Zwölf Vektoren, neu |

## Was v0.4 zusichert und was nicht

**Größe: zugesichert, je Eingabe, nicht im Mittel.** `len(encode(x)) ≤
ceil(4·len(x)/3)` für jede Eingabe und beide Profile, ohne Ausnahme (§9.4).
Der Beweis ist ein Satz: jeder Block nimmt die kürzeste von drei Formen, und
Base64 ist eine davon.

**Bytegleichheit: zugesichert.** `encode(x, profil)` ist eine Abbildung, die
je Block aus 48 Bytes und einer Maske eine Ausgabe bestimmt. Es gibt nichts,
worüber zwei Encoder verschiedener Meinung sein könnten (§11).

**Durchsatz: gemessen, nicht zugesichert.** §13.

**Das Wire-Format: nicht zugesichert.** Siehe oben.

## 0. Positionierung (nicht normativ)

### 0.1 Ein Format, ein Encoder

Base65t ist für den Fall gedacht, in dem jemand **unsicher** ist: er hat
etwas, das ohnehin schon Text ist, und muss es durch einen Kanal schicken, der
Bytes akzeptieren muss. Base64 kann er, Base64 versteht jeder, Base64 ist
niemals falsch. Alles, was er über ein anderes Verfahren erst lernen muss,
ist ein Grund, es bleiben zu lassen.

Deshalb gibt es genau eine parameterlose `encode`-Funktion, die Bytes nimmt
und Bytes liefert. Es gibt keine Presets, keine Modi, keine Schwellwerte. Der
Encoder ist in einem Satz erklärt: **48 Bytes Text bleiben Text, sonst
Base64, und dazwischen sagt eine Maske, was was ist.**

Zwei Parameter bleiben, und keiner davon ist eine Wahl über die Kodierung:

* **Das Profil** (§7) ist eine Aussage über den *Container*, nicht über den
  Strom, und aus dem Strom nicht ableitbar (§7.2). Der Default ist U.
* **`encode_base64url`** (§9.3) ist kein Modus des Formats, sondern der Ausgang
  aus ihm: für einen Aufrufer, der ein Geheimnis trägt und nichts davon im
  Klartext stehen haben will (§14), und für einen, der nur Base64URL sprechen
  darf.

| Einsatz | Profil | worauf es ankommt |
|---------|--------|-------------------|
| URL-Query | U | URL-Sicherheit ohne Prozent-Encoding |
| Cookie-Wert | U | `cookie-octet`-Konformität (§7.1) |
| HTTP-Header | U oder T | ASCII, keine Trennzeichen |
| Cache-/Dedup-Key | wie der Container | Bytegleichheit (§11) |
| Log-Feld, JSON-String | T | dort bleibt der Text lesbar (§13.4) |
| Token mit Geheimnisanteil | — | `encode_base64url`, keine Klartext-Leaks (§14) |

### 0.2 Was Base65t *ist*, in einem Satz

> Base64URL in Blöcken von 48 Bytes, wobei ein Block seine Bytes roh tragen
> darf und ein 65. Zeichen sagt, ob und welche.

### 0.3 Was „ein Dekoder für alles" genau heißt

> Ein konformer `decode()` nimmt einen Octet-Stream und ein Profil entgegen und
> benötigt **keinen weiteren Parameter**. Alphabetvariante (§5.2) und Padding
> (§5.3) werden aus dem Strom selbst bestimmt und im Ergebnis gemeldet.

### 0.4 Warum nicht „Base85N mit URL-Alphabet"

Die *unreserved*-Menge von RFC 3986 umfasst 66 Zeichen. Eine Radix-85-Kodierung ist
darin darstellbar, aber ein Passthrough vom Typ Base85N braucht zusätzlich
Spender-Zeichen — dafür bleibt kein Spielraum, sobald 64 Zeichen für den
Binärkern gebunden sind. Base65t geht den entgegengesetzten Weg: ein Kern, der
**exakt Base64URL ist**, plus ein Diskriminator. Nur daraus folgt die
Superset-Eigenschaft aus §5.2.

### 0.5 Wo Base65t in der Familie steht (nicht normativ)

Base65t ist der **Wegbereiter**, nicht der Höhepunkt. Base85N führt denselben
Durchreich-Gedanken weiter und ist dichter; Base91z komprimiert. Beide
verlangen vom Aufrufer, ein Verfahren zu lernen, das er noch nicht kennt.
Base65t verlangt das nicht: der Kern *ist* Base64URL, die Ausgabe ist niemals
größer, und die einzige Frage, die bleibt, ist das Profil. Der Mehrwert ist
entsprechend klein — auf kurzen Werten 21 %, über den Korpus 1,4 % in
Profil U und 5 % in T — aber die Kosten sind es auch.

## 1. Zielsetzung

1. **Nie schlechter als Base64** (§9.4).
2. Text durchreichen, auch wenn Satzzeichen dazwischenstehen (§6, §13.4).
3. Lesbar bleiben.
4. **Kein Escaping** — auch nicht für `~`.
5. **Abwärtskompatibel lesen** — jeder kanonische Base64- oder Base64URL-Strom,
   gepaddet oder nicht, dekodiert zu denselben Bytes (§5.2, §5.3). Normativ.
6. **Selbstbestimmend im Strom** — Alphabet und Padding werden erkannt,
   nicht konfiguriert (§0.3).
7. Bytegleich reproduzierbar (§11).
8. **Zustandslos.** Kein Block hängt von einem anderen ab (§4).

### 1.1 Die Kompatibilität ist asymmetrisch

| Richtung | Gilt? |
|----------|-------|
| Base65t-Dekoder liest Base64URL, ungepaddet | **ja**, normativ |
| Base65t-Dekoder liest Base64URL, gepaddet | **ja**, normativ, §5.3 |
| Base65t-Dekoder liest klassisches Base64 (`+`/`/`), gepaddet oder nicht | **ja**, normativ, §5.2/§5.3 |
| **Base64-Dekoder liest Base65t** | **nein** — `~` ist nicht im Alphabet |
| **Base65t v0.4 liest v0.1 bis v0.3** | **nein** — anderes Wire-Format |

Base65t ist ein *Superset auf der Leseseite* gegenüber Base64. Migrationspfad:
erst Dekoder ausrollen, später Encoder umstellen.

**Kanonizität der Eingabe.** Die Aussage gilt für *kanonische* Ströme. Ein
Base64-Strom mit gesetzten Restbits (`YWxpY2V`) wird mit `E_NONZERO_TAIL`
abgewiesen — auch dann, wenn eine permissive Base64-Bibliothek ihn akzeptiert
hätte. Das ist Absicht und gehört in den Differential-Fuzzing-Korpus (§16.2)
als *erwartete Abweichung*.

## 2. Nicht-Ziele

* **Kein Kompressionsformat.** Ab ca. 1 KB Text schlägt `gzip` + Base64 deutlich.
* **Kein Dichte-Rekord.** Z85, basE91, Base85N sind binär dichter.
* **Kein Durchsatz-Rekord.** Base64 ist der Maßstab, und Dichte wird nie gegen ihn
  eingetauscht.
* **Kein Sicherheitsmechanismus.**

## 3. Notation

* `byte` = Byte der Nutzdaten. `char` = Zeichen des Ausgabestroms.
* **Base65t erzeugt einen Octet-Stream.** In beiden Profilen ist jedes Oktett
  druckbares ASCII.
* Kernalphabet nach Base64URL (RFC 4648 §5): 0–25 `A`–`Z`, 26–51 `a`–`z`,
  52–61 `0`–`9`, 62 `-`, 63 `_`.
* Das 65. Zeichen ist `~` (U+007E), nicht Teil des Alphabets, ohne Wert.
* Bitreihenfolge MSB-first. MUSS / DARF NICHT / SOLLTE nach RFC 2119.

**Alphabetzeichen.** Ein Oktett heißt *Alphabetzeichen*, wenn der Dekoder es als
Wert 0–63 interpretiert: die Zeichen von Base64-Läufen und die Maskenzeichen
— **nicht** die rohen Bytes eines Blocks. Tragend für §5.4.

## 4. Streamstruktur

```
Stream      := Block*
Block       := Base64Block | RawBlock | MaskBlock
Base64Block := <64 Alphabetzeichen>                        # 48 Bytes
RawBlock    := "~~" <48 rohe Bytes>
MaskBlock   := "~" Mask <L rohe Bytes> <Base64 der übrigen 48−L Bytes>
Mask        := <8 Alphabetzeichen>                          # 48 Bit
```

**Die Eingabe wird an absoluten Offsets `k · 48` geschnitten.** Jeder Block
außer dem letzten deckt genau 48 Eingabebytes ab; der letzte deckt die
restlichen `m ≤ 48` und ist entsprechend kürzer:

* ein Base64-Tail hat `ceil(4m/3)` Zeichen,
* ein Raw-Tail hat `2 + m` Zeichen und läuft bis zum Stromende,
* ein Masken-Tail hat `1 + 8 + L + ceil(4(m−L)/3)` Zeichen, und die Maskenbits
  `≥ m` MÜSSEN 0 sein (§6.2).

**Blöcke sind unabhängig (normativ).** Die Kodierung eines Blocks hängt nur
von seinen eigenen Bytes ab. Daraus folgt alles, was dieses Format von seinen
Vorgängern unterscheidet: der Encoder hat keinen Zustand, der Dekoder kennt
die Länge eines Blocks, bevor er ihn liest, und ein Strom lässt sich an jeder
Blockgrenze teilen und wieder zusammensetzen.

**Base64-Blöcke kacheln.** 48 Bytes sind 16 Quanten, also endet ein
Base64-Block auf einer Quantengrenze, und zwei aufeinanderfolgende
Base64-Blöcke sind ein Base64-Lauf. Deshalb ist ein reiner Base64-Strom ein
gültiger Strom, und deshalb DARF ein Dekoder aufeinanderfolgende
Base64-Blöcke als einen Lauf lesen (§10.1).

**Warum 48.** Drei Bedingungen zugleich: durch 3 teilbar, damit Base64-Blöcke
kacheln; durch 6 teilbar, damit die Maske aus ganzen Zeichen besteht; und groß
genug, dass die zwei Marker-Zeichen eines Raw-Blocks vier Prozent davon sind
und nicht ein Drittel — bei sechs Bytes je Block spart die rohe Form genau
nichts (§9.1). Größere Blöcke sparen wenig zusätzlich und schieben mehr Bytes
in den Base64-Anhang eines Maskenblocks.

Ein leerer Strom ist gültig und dekodiert zu null Bytes.

## 5. Base64-Läufe

Base64URL. Sei `n` die Zeichenzahl ohne Padding:

| `n mod 4` | Bytes im letzten Quantum | Gültig |
|-----------|--------------------------|--------|
| 0 | 3 | ja |
| 2 | 1 | ja |
| 3 | 2 | ja |
| 1 | — | **nein** (`E_ALIGN`) |

**Kanonizität:** ungenutzte Bits im letzten Zeichen MÜSSEN 0 sein
(`E_NONZERO_TAIL`). Das gilt für jeden Base64-Lauf, auch für den Anhang eines
Maskenblocks, der für sich allein ein Lauf ist.

### 5.1 Encoder-Alphabet

Ein Encoder MUSS genau ein Alphabet je Aufruf verwenden und DARF innerhalb eines
Stroms nicht wechseln.

| Alphabet | 62 / 63 | Zweck |
|----------|---------|-------|
| **URL** (Default) | `-` / `_` | alles Neue |
| **Classic** (Opt-in) | `+` / `/` | nur Interop |

Classic ist nicht URL-sicher und DARF NICHT als Default angeboten werden. Der
Encoder erzeugt **niemals** Padding (§5.3).

### 5.2 Permissive Dekodierung (normativ)

Jeder konforme Dekoder MUSS beide Alphabete akzeptieren: `-`/`+` → 62, `_`/`/` → 63.
Gilt für Base64-Läufe **und** Maskenzeichen.

### 5.3 Padding (normativ)

> **Regel P.** Ein Base64-Lauf, der **am Stromende** endet, DARF mit 1 oder 2 `=`
> abgeschlossen sein. Der Dekoder MUSS diese akzeptieren. „Strom" heißt der
> ganze Oktett-Strom.

| `k` (`=`-Anzahl) | erforderlich |
|------------------|--------------|
| 0 | `n mod 4 ∈ {0, 2, 3}` |
| 1 | `n mod 4 == 3` |
| 2 | `n mod 4 == 2` |

Jede andere Kombination → `E_PADDING`. `=` an jeder anderen Position →
`E_CHARSET`.

**Implementierungsfalle.** Padding DARF NICHT vorab vom Stromende gestrippt
werden. In Profil T ist `=` legales rohes Byte, und ein Raw-Tail läuft bis
zum Stromende: `~~a=b=` sind vier Bytes Nutzlast (TV10).

### 5.4 Alphabet-Konsistenz (Regel A, normativ)

> **Regel A.** Ein Strom DARF NICHT beide Alphabetvarianten mischen. Enthält die
> Menge der **Alphabetzeichen** sowohl ein Zeichen aus {`+`,`/`} als auch eines aus
> {`-`,`_`} → `E_MIXED_ALPHABET`.

Regel A betrifft ausschließlich Alphabetzeichen. Rohe Bytes zählen nicht mit
— in Profil U enthält fast jeder Raw-Block `-` oder `_`. Wer den Gesamtstrom
scannt, weist gültige Ströme ab (TV7).

### 5.5 Meldung und strikte Variante (normativ)

Ein `decode()`-Ergebnis MUSS enthalten:

```
alphabet_seen : { none, url, classic }
padding_seen  : bool
```

Zusätzlich MUSS `decode_url_strict` angeboten werden (weist `classic` mit
`E_NON_URL_ALPHABET` ab).

## 6. Der Maskenblock

```
MaskBlock := "~" Mask <L rohe Bytes> <Base64 der übrigen Bytes>
```

Der Block, der dieses Format von seinen Vorgängern unterscheidet. Ein Block,
in dem manche Bytes das Profil erfüllen und manche nicht, wird nicht als
Ganzes nach Base64 geschickt. Die Maske sagt je Byte, ob es roh dasteht; die
rohen Bytes folgen in Eingabereihenfolge, dann als ein Base64-Lauf die
übrigen, ebenfalls in Eingabereihenfolge.

### 6.1 Die Maske

Acht Alphabetzeichen, 48 Bit, **ein Bit je Eingabebyte des Blocks**. Zeichen
`j` trägt die Bytes `6j` bis `6j+5`; das erste davon steht im **höchsten** Bit
des Zeichenwerts, damit die Maske sich von links nach rechts liest wie die
Bytes, die sie beschreibt.

```
Zeichenwert(j) = Σ_{t=0..5}  legal(6j + t) << (5 − t)
```

Beispiel: die ersten sechs Bytes `the qu` in Profil U sind legal, illegal
(Leerzeichen), legal, legal, legal, legal → `110111`? Nein: `t h e _ q u` ist
`1 1 1 0 1 1` = 59 = `7`. TV5 zeigt den ganzen Block.

### 6.2 Zulässigkeit

* Ein Bit DARF nur für ein Byte gesetzt sein, das das Profil zulässt
  (`E_PROFILE` beim Dekodieren, weil der Dekoder die rohen Bytes prüft).
* In einem Tail von `m < 48` Bytes MÜSSEN die Bits `≥ m` null sein
  (`E_MASK`). Der Dekoder bestimmt `m` als Summe aus gesetzten Bits und
  dekodierten Base64-Bytes (§10.1).
* Eine Maske mit allen 48 Bits gesetzt ist gültig, aber kein Encoder schreibt
  sie: die rohe Form ist kürzer (§9.0). Eine Maske ohne gesetztes Bit ebenso.

## 7. Profile

| Profil | Erlaubte rohe Bytes | URL-Query direkt? |
|--------|---------------------|-------------------|
| **U** (Default) | RFC-3986-*unreserved* (66 Zeichen) | **ja** |
| **T** | ASCII 0x20–0x7E ohne `"` und `\` (93 Zeichen) | nein |

Ein profilwidriges Byte ist kein Sonderfall: es landet im Base64-Anhang des
Maskenblocks oder, wenn zu wenige Bytes legal sind, der Block als Ganzes in
Base64.

**Profil T** ist JSON-String-sicher, **nicht** CSV-struktursicher und **nicht**
URL-sicher. **Und es enthält das Leerzeichen** (0x20): eine
whitespace-getrennte Logzeile muss einen T-Wert quoten, ein
`key=value`-Format nicht. Gefunden vom Container-Test aus §16.6.

### 7.1 Cookie-Konformität von Profil U (bewiesen, nicht gemessen)

RFC 6265 §4.1.1 definiert:

```
cookie-octet = %x21 / %x23-2B / %x2D-3A / %x3C-5B / %x5D-7E
```

Das Alphabet von Profil U — 62 Alphanumerische plus `-` (0x2D), `.` (0x2E),
`_` (0x5F), `~` (0x7E) — liegt vollständig in diesen Bereichen. Alle 66
Zeichen geprüft, keine Ausnahme. Die Aussage folgt aus der ABNF und ist damit
**beweisbar, nicht empirisch**. Ob reale Cookie-Parser sich an die ABNF
halten, ist die schwächere, empirische Frage; Pythons `http.cookies` tut es
(§16.6).

### 7.2 Warum das Profil Parameter bleibt

Das Profil ist aus dem Strom nicht ableitbar: ein Strom, dessen rohe Bytes
zufällig nur *unreserved* sind, ist unter U und T identisch gültig. Es
beschreibt die Erwartung des **Containers**, nicht eine Eigenschaft des
Stroms.

## 8. Framed Mode — **zurückgezogen**

Siehe `docs/history/`. Ein Blockformat mit festen Blockgrenzen braucht keinen
zweiten Modus für wahlfreien Zugriff; wer ihn will, indiziert Blockanfänge,
und das ist ein Erweiterungskandidat (§17), keine Formatfrage.

## 9. Encoder

### 9.0 Grundprinzip (normativ)

> Für jeden Block bestimmt der Encoder die Maske — welche Bytes das Profil
> zulässt — und schreibt den Block in der **kürzesten** der drei Formen aus
> §4. Sind zwei Formen gleich lang, MUSS er die nehmen, in der **mehr Bytes
> roh** dastehen.

Das ist die ganze Regel. Die Länge jeder Form ist eine Funktion von `m` und
der Anzahl gesetzter Bits `L`:

```
Base64:  ceil(4m/3)
Raw:     m + 2                          nur wenn L = m
Mask:    1 + 8 + L + ceil(4(m−L)/3)
```

Der Tie-Break trifft genau zwei Fälle: die Maske gegen Base64 bei `L = 27`
in einem vollen Block, und Raw gegen Base64 bei Tails von 4, 5 und 6 Bytes.
In beiden Fällen kostet die Wahl nichts, und Lesbarkeit ist, wofür das Format
da ist.

Damit prüft ein Testvektor Bytes statt Längen, und `docs/vectors.json` tut das
über 183 Vektoren.

### 9.1 Was die Formen kosten

Für einen vollen Block gegen dessen 64 Base64-Zeichen:

| legale Bytes `L` | Raw | Maske | Gewinner |
|--:|--:|--:|---|
| 48 | 50 | 57 | Raw, 78 % |
| 40 | — | 60 | Maske, 94 % |
| 30 | — | 63 | Maske, 98 % |
| 27 | — | 64 | Maske, Gleichstand |
| 26 | — | 65 | Base64 |
| 0 | — | 73 | Base64 |

Die Maske zahlt ein Bit je Byte, also ein Sechstel Zeichen, plus ein Zeichen
Marker. Ein rohes Byte spart gegenüber Base64 ein Drittel Zeichen. Der
Maskenblock lohnt sich deshalb, sobald mehr als 27 von 48 Bytes legal sind,
und er kann nie mehr als 7/64 gegenüber Base64 sparen. Die rohe Form hat
diese Grenze nicht: sie zahlt zwei Zeichen je 48 Bytes und kommt auf 78 %.

Das ist auch die Antwort darauf, warum ein Byte je Block und nicht ein Bit je
Byte für *alle* Blöcke: das häufigste Muster — alles legal — bekommt den
kürzesten Code. Eine winzige Entropiekodierung des Maskenraums.

**Was gegenüber der Segmentfassung verloren geht**, ist langer sauberer
Text: dort kostete ein Literal-Header zwei bis vier Zeichen je 4158 Bytes,
hier kosten zwei Zeichen je 48. Prosa in Profil T kommt auf 88 % statt 80 %.
Was gewonnen wird, ist gemischter Text: dieselbe Prosa in Profil U zeigt
76 % ihrer Bytes statt 17 %, weil die Maske je Byte zahlt statt je Lauf und
ein Lauf von fünf Bytes zwischen zwei Leerzeichen keinen Header wert war.
§13.4 hat die Tabelle.

### 9.2 Optimale Segmentierung — **entfällt**

Es gibt nichts zu segmentieren. Der Encoder der Segmentfassung war ein
Programm über Läufe datenabhängiger Länge; dieses ist eine Abbildung über
Blöcke fester Länge, und §9.0 ist die vollständige Beschreibung.

### 9.3 Einstiegspunkte

| Funktion | Was sie tut | Profil |
|---|---|---|
| `encode(x)` | die Kodierung | U |
| `encode(x, profil)` | dieselbe, im Profil, das der Container verlangt | U oder T |
| `encode_base64url(x)` | Base64URL und sonst nichts (§14) | — |

Ein Aufruf ohne Parameter MUSS `encode` + Profil U liefern. Bibliotheken
SOLLTEN genau eine parameterlose `encode`-Funktion exportieren.

### 9.4 Nie-schlechter-Garantie (normativ)

```
len(encode(x, profil)) <= ceil(4 * len(x) / 3)
```

**Je Eingabe, nicht im Mittel, ohne Ausnahme.** Beweis: jeder Block nimmt die
kürzeste von drei Formen, und Base64 ist eine davon; Base64-Blöcke kacheln,
also ist die Summe der Base64-Formen genau `ceil(4n/3)`. ∎

**Schärfer:** wo kein Block eine andere Form nimmt, schreibt der Encoder nicht
nur gleich viele Zeichen wie Base64URL, sondern **dieselben Bytes**.

**Geltungsbereich.** Die Länge des kodierten Stroms in Oktetten, nicht
Transport- oder Container-Overhead.

### 9.5 Segmentwechselrate — **entfällt**

Es gibt keine Segmentwechsel. Ein Block hat eine Form, und der Übergang zum
nächsten Block ist eine Position, die der Dekoder kennt, bevor er hinschaut.

### 9.6 Kopfentscheidung — **entfällt**

Die Segmentfassung entschied am Dateianfang, ob sich Suchen lohnt, weil
Suchen die teuerste Operation ihres Encoders war. Dieser Encoder sucht nicht.
Auf komprimierten Daten ist jeder Block ein Base64-Block, und die Maske,
die das feststellt, kostet weniger als das Schreiben des Blocks (§13).

## 10. Dekoder

### 10.1 Ablauf

```
pos := 0 ; alphabet_seen := none ; padding_seen := false
while pos < len:
    if stream[pos] != '~':
        # Base64-Lauf: jeder Block, der mit einem Alphabetzeichen beginnt,
        # ist 64 Zeichen lang; der letzte, was übrig ist. Blöcke kacheln,
        # also DARF der Lauf als Ganzes dekodiert werden.
        end := pos
        while end < len and stream[end] != '~': end := min(end + 64, len)
        emit base64_decode(stream[pos..end], padding_erlaubt = (end == len))
        pos := end
    elif pos + 1 == len:                                  -> E_TRAILING_TILDE
    elif stream[pos+1] == '~':
        # Raw-Block: 48 Bytes, oder was übrig ist.
        end := min(pos + 2 + 48, len)
        prüfe: alle Bytes stream[pos+2..end] profil-legal    sonst E_PROFILE
        emit stream[pos+2..end] ; pos := end
    else:
        # Maskenblock.
        prüfe: pos + 9 <= len                                sonst E_TRUNCATED
        prüfe: stream[pos+1..pos+9] Alphabetzeichen          sonst E_CHARSET   # (1)
        note_alphabet für jedes davon                                           # (2)
        mask := 48 Bit aus den acht Zeichen (§6.1) ; pos += 9
        L := popcount(mask)
        prüfe: pos + L <= len                                sonst E_TRUNCATED
        clear := stream[pos..pos+L]
        prüfe: alle Bytes von clear profil-legal             sonst E_PROFILE
        pos += L
        full := ceil(4·(48 − L)/3)
        tail := (len − pos <= full)
        n := tail ? len − pos : full
        rest := base64_decode(stream[pos..pos+n], padding_erlaubt = tail)
        pos += n
        m := L + len(rest)
        prüfe: m <= 48 and (mask >> m) == 0                  sonst E_MASK       # (3)
        emit: für i in 0..m: bit i gesetzt ? nächstes Byte aus clear : aus rest

base64_decode(seg, padding_erlaubt):                       # §5, §5.3
    k := padding_erlaubt ? Anzahl '=' am Ende (max 2) : 0
    n := len(seg) − k
    prüfe: k == 0 ∨ (k == 1 ∧ n mod 4 == 3) ∨ (k == 2 ∧ n mod 4 == 2)
                                                             sonst E_PADDING
    if k > 0: padding_seen := true
    prüfe: n mod 4 != 1                                      sonst E_ALIGN
    prüfe: alle n Zeichen Alphabetzeichen                    sonst E_CHARSET
    note_alphabet für jedes Zeichen mit Wert 62/63                              # (2)
    prüfe: Restbits des letzten Quantums == 0                sonst E_NONZERO_TAIL
    return Bytes

note_alphabet(c):
    if c in {'+','/'}:  if alphabet_seen == url     -> E_MIXED_ALPHABET
                        else alphabet_seen := classic
    if c in {'-','_'}:  if alphabet_seen == classic -> E_MIXED_ALPHABET
                        else alphabet_seen := url
```

**(1)** Ohne diese Prüfung wird `value()` auf wertlosen Zeichen aufgerufen —
undefiniert oder Lookup außerhalb der Tabelle. **(2)** implementiert Regel A.
**(3)** Ein voller Block hat `m = 48`, und dann ist `mask >> 48` ohnehin null;
die Prüfung greift im Tail, wo die Maske Bits für Bytes tragen könnte, die es
nicht gibt.

**Es gibt keine Suche.** Der Dekoder liest nie „bis zum nächsten `~`"; jede
Länge steht fest, bevor er ein Nutzbyte anfasst. Das ist mehr als eine
Bequemlichkeit: ein Byte `~` in einem Raw-Block oder im Klartext eines
Maskenblocks ist Nutzlast, und ein Dekoder, der danach sucht, liest ihn
falsch (TV3).

**Warum der Tail eindeutig ist.** Ein Masken-Tail mit `m < 48` Bytes und
derselben Maske ist immer kürzer als der volle Block, weil `ceil(4x/3)` in `x`
streng wächst. „Es bleiben weniger Zeichen als ein voller Block braucht" ist
deshalb die ganze Tail-Erkennung. Für Raw- und Base64-Blöcke ebenso.

### 10.2 Einstiegspunkt

```
decode(stream, profile)
decode_url_strict(stream, profile)      # weist '+' und '/' mit E_NON_URL_ALPHABET ab
```

### 10.3 Framed Mode — **zurückgezogen**

Siehe §8.

### 10.4 Fehlerfälle

| Code | Bedingung |
|------|-----------|
| `E_TRAILING_TILDE` | Strom endet mit einem einzelnen `~` |
| `E_TRUNCATED` | Maskenblock endet vor dem Ende der Maske oder der Klartext-Bytes |
| `E_PROFILE` | rohes Byte außerhalb des Profil-Alphabets |
| `E_ALIGN` | Base64-Lauflänge `mod 4 == 1` |
| `E_NONZERO_TAIL` | Restbits im letzten Quantum ≠ 0 |
| `E_CHARSET` | kein Alphabetzeichen an Alphabetposition (inkl. `~` in einem Base64-Lauf, Maskenposition, `=` außerhalb des Stromendes) |
| `E_PADDING` | Regel P verletzt |
| `E_MIXED_ALPHABET` | Regel A verletzt |
| `E_NON_URL_ALPHABET` | nur `decode_url_strict` |
| `E_MASK` | Maske beansprucht ein Byte hinter dem Blockende |

**Allokationsgrenzen.** Es gibt im Strom keine Länge, die ein Sender wählt.
Eine Maske kann höchstens 48 Bytes benennen, ein Raw-Block hat höchstens 48,
ein Base64-Lauf ergibt drei Bytes je vier Zeichen. Daraus folgt: die
Spezifikation braucht **kein protokollseitiges Limit für einzelne Blöcke**,
und die Klasse der Einzelallokations-Bugs, die §14 der Segmentfassung als
ihre eine Schwäche gegenüber Base64 nannte, existiert nicht.

Daraus folgt **nicht**, dass gar kein Limit nötig wäre. Die Zahl der Blöcke
ist unbegrenzt. Implementierungen SOLLTEN Gesamtgrößen- und Laufzeitlimits
anbieten.

## 11. Kanonizität und Signaturen

**Der Encoder ist eine Abbildung** (§9.0): je Block bestimmen 48 Bytes und
das Profil die Ausgabe, und die Blöcke sind unabhängig. Zwei konforme Encoder
schreiben für dieselbe Eingabe und dasselbe Profil dieselben Bytes. Das reicht
für Cache-Keys, Dedup-Keys und Content-Adressen, wo dieselbe Seite erzeugt
und vergleicht.

Kanonisch ist das *Format* trotzdem nicht, aus zwei Gründen. Erstens ist das
**Profil eine Wahl**: derselbe Input ergibt unter U und T verschiedene Ströme.
Zweitens akzeptiert der **Dekoder Formen, die kein Encoder schreibt**: das
Classic-Alphabet (§5.2), Padding (§5.3), einen Maskenblock, wo Raw oder Base64
kürzer wäre (§6.2). Ein Dritter kann denselben Strom umschreiben, ohne die
dekodierten Bytes zu ändern.

> **Regel:** Signiere, hashe und vergleiche niemals die Ausgabe von `encode`.
> Signiere die **dekodierten Bytes**. `decode(encode(x)) == x` gilt immer.

**Die Ordnung `B < L < S`** der Segmentfassung gibt es nicht mehr. Sie war
nötig, weil dort mehrere Segmentierungen gleich lang sein konnten und eine
davon gewählt werden musste. Hier gibt es je Block drei Formen, und §9.0 sagt
in einem Satz, welche.

## 12. Dichte

**Exakt**, aus §9.1:

| Eingabe | Base64 | **Base65t** |
|---------|--------|-------------|
| Rein binär | 1,333 | **1,333** — dieselben Bytes |
| Rein profil-legaler Text | 1,333 | **1,0417** — `50/48`, jeder Block roh |
| Ein Block mit `L` legalen Bytes, `27 ≤ L < 48` | 1,333 | `(9 + L + ceil(4(48−L)/3)) / 48` |

**Gemessen** über den Korpus von binary2textbench (69 Proben, `--example
gain`), Größe gegen ungepaddetes Base64:

| | Profil U | Profil T |
|---|--:|--:|
| Summe über alle Proben | 98,65 % | 95,03 % |
| Proben besser als 95 % | 46 % | |
| Proben besser als 99 % | 65 % | |
| von Base64 nicht zu unterscheiden (≥ 99,9 %) | 29 % | |

Die Verteilung ist keine Kurve, sondern zwei Populationen: Werte, die schon
Text sind, bei 78 bis 79 %; komprimierte und zufällige Daten bei 100 %; und
dazwischen gemischter Text, der mit der Maske zwischen 94 und 99 % liegt.

**Die Segmentfassung zum Vergleich**, dieselben Proben: Profil U 98,57 %,
Profil T 93,60 %. Das Blockformat ist in U um 0,08 Punkte und in T um 1,4
Punkte größer. Was es dafür bekommt, steht in §13.

## 13. Performance

Gemessen gegen die Base64-Implementierung des Benches, die im selben Prozess
lebt und vom selben Compiler gebaut wurde. Alles einthreadig, bestes von fünf
Läufen, Base64 = 100 %.

### 13.1 Die drei Blockformen, isoliert

Auf erzeugten Eingaben, die je nur eine Form erzeugen, gegen die Base64-Schleife
der Referenzimplementierung selbst (`--example prof`, im Verlauf der
Entwicklung):

| Blockform | Kodieren | Dekodieren |
|---|--:|--:|
| Raw (`~~` + 48 Bytes) | 97 % | 94 % |
| Base64 | 190 % | 100 % |
| Maske | 290 % | 320 % |

Raw-Blöcke sind ein `memcpy` je Richtung und laufen auf Base64-Niveau.
Base64-Blöcke dekodieren auf Parität, weil aufeinanderfolgende Blöcke als ein
Lauf gelesen werden; beim Kodieren kostet die Maske, die feststellt, dass der
Block Base64 wird, ein Fünftel Nanosekunde je Byte, und das ist der ganze
Abstand. **Der Maskenblock ist der eine Pfad, der dreimal Base64 kostet**, und
zwar auf beiden Seiten, weil er dreimal so viel tut: die Maske schreiben oder
lesen, 48 Bytes nach zwei Zielen trennen oder aus zweien zusammensetzen, und
den Rest als Base64.

### 13.2 Das Durchsatz-Kriterium

> **Durchsatz ist ein Ziel, Größe ist eine Zusicherung.** Eine Änderung DARF
> die Zusicherung aus §9.4 und die Bytegleichheit aus §11 nicht antasten.
> Innerhalb dieser Schranke SOLL sie den Durchsatz verbessern.

Ein Encoder oder Dekoder ist nicht deshalb unkonform, weil er langsamer ist als
Base64. Er ist es, wenn er die falschen Bytes schreibt.

### 13.3 Große Dateien

Silesia und Korpusdateien, MiB/s und Base65t als Anteil an der Base64-Zeit
(`--example large`):

| Datei | Profil | Größe | Base64 enc | Base65t enc | enc | Base64 dec | Base65t dec | dec |
|---|---|--:|--:|--:|--:|--:|--:|--:|
| `x-ray` | U | 100,0 % | 692 | 695 | **100 %** | 1149 | 1300 | **88 %** |
| `mozilla` | U | 99,0 % | 544 | 474 | **115 %** | 764 | 667 | **115 %** |
| `countries.json` | U | 99,3 % | 821 | 577 | **142 %** | 1183 | 812 | **146 %** |
| `dickens` | U | 95,6 % | 671 | 398 | **169 %** | 1099 | 341 | **322 %** |
| `xml` | U | 97,1 % | 721 | 398 | **181 %** | 1163 | 363 | **320 %** |
| `dickens` | T | 87,9 % | 665 | 494 | **135 %** | 1081 | 482 | **224 %** |
| `xml` | T | 85,2 % | 730 | 602 | **121 %** | 1142 | 531 | **215 %** |
| `countries.json` | T | 93,5 % | 768 | 422 | **182 %** | 1152 | 328 | **351 %** |

Die Tabelle liest sich über §13.1: je mehr Maskenblöcke eine Datei erzeugt,
desto näher liegt sie am Dreifachen. Binärdaten sind Base64-Blöcke und laufen
auf Parität oder darunter. Prosa in Profil U ist fast durchgehend Maske und
kostet das Drei­fache beim Dekodieren. Dieselbe Prosa in Profil T ist zu
neun Zehnteln roh, und die verbleibenden Maskenblöcke — die mit einem
Anführungszeichen — kosten das Doppelte.

**Gegenüber der Segmentfassung:** dort kostete das exakte Programm auf
`dickens` 1137 % beim Kodieren, hier 169 %; auf `xml` 922 %, hier 181 %. Der
Dekoder der Segmentfassung war schneller (165 % auf `dickens`), weil er
lange Base64-Läufe las und wenige Literale; dieser liest Maskenblöcke, und
die kosten. Was er dafür liefert, steht in §13.4.

### 13.4 Kurze Werte, und was lesbar bleibt

Die 55 kurzen Proben, Profil U, Größe gegen `ceil(4n/3)`, Zeit gegen die
Base64 des Benches (`--example short`):

| Probe | Bytes | Form | Größe | Kodieren | Dekodieren |
|---|--:|---|--:|--:|--:|
| SHA-512-Digest, hex | 128 | roh | 78 % | **68 %** | **80 %** |
| JWT, drei Segmente | 155 | roh | 79 % | **64 %** | **80 %** |
| Session-ID, 40 alnum | 40 | roh | 78 % | **62 %** | **81 %** |
| UUID v4 | 36 | roh | 79 % | **64 %** | **87 %** |
| Kreditkartennummer | 16 | roh | 82 % | **71 %** | 110 % |
| Vor- und Nachname | 12 | Base64 | 100 % | 95 % | 151 % |
| IPv6-Adresse | 28 | Base64 | 100 % | 109 % | 100 % |
| zufällige 64 Bytes | 64 | Base64 | 100 % | 117 % | 92 % |
| Logzeile | 93 | Maske | 94 % | 170 % | 239 % |
| SQL-Statement | 118 | Maske | 96 % | 166 % | 209 % |
| JSON-Datensatz | 92 | Maske | 98 % | 181 % | 236 % |
| **alle 55 Proben, als Zeit** | | | | **98 %** | **123 %** |

Die Summenzeile ist nach Zeit gewichtet und erreicht Parität beim Kodieren.
Die Segmentfassung lag hier bei 355 %, weil ihr exaktes Programm auf jeder
Probe mit einem Leerzeichen das Acht- bis Zehnfache kostete; hier kostet
dieselbe Probe das Doppelte und ist dafür lesbar.

**Was lesbar bleibt**, Anteil der Bytes, die im Strom stehen wie in der
Eingabe (`--example clear`):

| Datei | Profil | Segmentfassung | **Blockformat** |
|---|---|--:|--:|
| Prosa (dickens) | U | 99 % / 17 % | **96 % / 76 %** |
| XML | U | 98 % / 21 % | **97 % / 66 %** |
| CSS | U | 92 % / 54 % | **96 % / 72 %** |
| JSON | T | 94 % / 47 % | **94 % / 84 %** |
| Prosa (dickens) | T | 80 % / 91 % | 88 % / 96 % |
| XML | T | 80 % / 92 % | 85 % / 97 % |

Das ist der Tausch, den §9.1 beschreibt, in Zahlen. Die Maske zahlt ein Bit
je Byte, unabhängig davon, wie die legalen Bytes verteilt sind, und
gemischter Text wird dreimal so lesbar. Langer sauberer Text zahlt zwei
Zeichen je 48 Bytes statt zwei je 4158 und verliert fünf bis acht Punkte
Größe. Wer ein Megabyte sauberen Text kodiert, hat einen Kompressor
verdient; wer eine Logzeile kodiert, hat sie jetzt lesbar.

## 14. Sicherheit

* **Der Dekoder parst keine angreifergewählte Länge.** Die Segmentfassung
  stand hier hinter Base64: ihr Dekoder las Längen bis 4158 aus dem Strom.
  Dieser liest eine Maske, und eine Maske kann nichts hinter ihrem eigenen
  Block adressieren. Was bleibt, ist dasselbe wie bei Base64: die
  Gesamtlänge der Eingabe.
* **Rohe Bytes lecken Struktur** — welche Bytes Text sind und welche nicht,
  ist im Strom sichtbar. Dafür ist `encode_base64url` da (§9.3); seine
  Ausgabe ist gewöhnliches Base64URL.
* **Zwei Auto-Erkennungen sind zwei Parser-Differential-Flächen:** Alphabet
  (§5.2) und Padding (§5.3). Gegenmaßnahmen: Regel A, Regel P,
  `alphabet_seen` / `padding_seen` und `decode_url_strict` (§5.5).
  Differential-Fuzzing ist Pflicht, nicht Kür.
* **Kein Padding-Orakel** — Padding wird nur validiert, nie erzeugt.
* **Malleability** ausgeschlossen auf Blockebene, reduziert auf Alphabet- und
  Padding-Ebene, **nicht** auf Profil- und Formebene (§11).
* Dekodierte Ausgabe ist **untrusted binary**, nicht Text.

## 15. Testvektoren

Zwölf Vektoren, jeder als Test in `rust/tests/vectors.rs`. Der maschinell
prüfbare Satz — 183 Einträge über beide Einstiegspunkte und beide Profile —
steht in `docs/vectors.json`. Die Vektoren der Segmentfassung liegen in
`docs/history/`; ihre *Eingaben* wurden übernommen, ihre Ströme nicht.

### TV1–TV4 — die drei Formen (Profil U)

| # | Eingabe | Strom | Länge | Base64 wäre |
|---|---------|-------|-------|-------------|
| TV1 | `alice.jones` | `~~alice.jones` | 13 | 15 |
| TV2 | `DE AD BE EF` + `session-eu-central` | `3q2-73Nlc3Npb24tZXUtY2VudHJhbA` | 30 | 30 |
| TV3 | `sub~alice~jones` | `~~sub~alice~jones` | 17 | 20 |
| TV4 | 100 × `a` | `~~` + 48 `a`, `~~` + 48 `a`, `~~aaaa` | 106 | 134 |

**Zu TV2.** Ein Tail von 22 Bytes mit 18 legalen: die Maske kostete
`9 + 18 + 6 = 33`, Base64 30. Die Segmentfassung schrieb diese Eingabe in
26 Zeichen; das Blockformat gibt das auf und bekommt dafür TV5.

**Zu TV3.** `~` in einem Raw-Block braucht nichts, weil die Blocklänge
feststeht. `hello~Alice` wird `~~hello~Alice`.

### TV5 — der Maskenblock

Fünfzig Bytes Englisch in Profil U. Der erste Block hat neun Leerzeichen und
einen Punkt; die Segmentfassung hätte ihn als zehn kurze Base64-Läufe
geschrieben, weil kein Lauf lang genug für ein Literal war.

```
Eingabe:  the quick brown fox jumps over the lazy dog. again
Strom:    ~777vvd73thequickbrownfoxjumpsoverthelazydog.agaICAgICAgICAgaW4
          ^ ^^^^^^^^ ^                                     ^           ^
          | Maske    | 39 legale Bytes                     | 9 Leerz.  | Tail "in"
```

63 Zeichen, Base64 wären 67. Das erste Maskenzeichen ist `7`: `t h e _ q u`
ist `1 1 1 0 1 1` = 59. In Profil T ist das Leerzeichen legal, der erste
Block wird roh, und der Strom ist `~~the quick … aga` + `aW4`, 53 Zeichen.

**TV5b, der Gleichstand.** 27 legale Bytes und 21 Leerzeichen: Maske 64,
Base64 64 → die Maske, mit `____4AAA`. 26 legale: Base64, byteweise.

### TV6 — Abwärtskompatibilität

| Strom | Bytes | `alphabet_seen` | `padding_seen` |
|-------|-------|-----------------|----------------|
| `PDw_Pz8-Pg` | `<<???>>` | url | false |
| `PDw/Pz8+Pg` | `<<???>>` | classic | false |
| `YWxpY2Uuam9uZXM` | `alice.jones` | none | false |
| `YWxpY2U=` | `alice` | none | true |

Ein Base64-Strom beliebiger Länge liest sich in Blöcken von 64 Zeichen, und
das ist unsichtbar, weil Base64-Blöcke kacheln.

### TV7 — Alphabet-Konsistenz

`PDw_Pz8+Pg` und `PDw/Pz8-Pg` → `E_MIXED_ALPHABET`. Maskenzeichen sind
Alphabetpositionen: die Maske `////4AAA` statt `____4AAA` liest denselben
Block im Classic-Alphabet, `_///4AAA` ist gemischt. Rohe Bytes zählen nicht:
`~~a+b/c-d_e` in Profil T ist `alphabet_seen = none`.

### TV8 — Maskenpositionen werden vor dem Lesen geprüft

`~=AAAAAAA`, `~AAAAAA~A` → `E_CHARSET`. `~` → `E_TRAILING_TILDE`. `~AAAA` →
`E_TRUNCATED`.

### TV9–TV10 — Padding

```
YWxpY2U=     -> "alice",  padding_seen
YWxpY2Uu     -> "alice.", kein Padding
YWxp==       -> E_PADDING
YWxpY2U==    -> E_PADDING
```

Ein `=` am Ende des 64. Zeichens eines Base64-Blocks, dem ein weiterer Block
folgt, ist `E_CHARSET`. TV10, Profil T: `~~a=b=` ist ein Raw-Tail und trägt
vier Bytes, davon zwei `=`; in Profil U ist es `E_PROFILE`.

### TV11 — Fehlerfälle

| Strom | Code |
|-------|------|
| `abcde` | `E_ALIGN` |
| `~` | `E_TRAILING_TILDE` |
| `~AAAA` | `E_TRUNCATED` |
| `~~a b` | `E_PROFILE` (Profil U) |
| `YWxp==` | `E_PADDING` |
| `YWxpY2V` | `E_NONZERO_TAIL` |
| `YW~x` | `E_CHARSET` |
| `~AAAAAAABa` | `E_MASK` |

Der letzte beansprucht Byte 47 in einem Tail, der ein Byte hat.

### TV12 — die Maske im Tail

30 Bytes, 27 legale, drei Leerzeichen: Maske `9 + 27 + 4 = 40`, Base64 40,
Gleichstand → Maske.

```
~____4AAA aaaaaaaaaaaaaaaaaaaaaaaaaaa ICAg
```

Derselbe Strom mit `B` statt `A` als achtem Maskenzeichen beansprucht Byte 47
und ist `E_MASK`.

## 16. Konformitätsnachweise

Eine Implementierung gilt als konform, wenn sie die drei folgenden
Eigenschaften belegt:

1. **`decode(encode(x)) == x`** für beide Profile, über einen Fuzzing-Korpus.
2. **`decode(base64(x)) == x`** und **`decode(base64url(x)) == x`** für alle
   kanonischen Eingaben, gepaddet und ungepaddet — per Differential-Fuzzing gegen die
   Standard-Base64-Bibliothek der jeweiligen Sprache. Erwartete Abweichungen
   (`E_NONZERO_TAIL`, §1.1) gehören als solche in den Korpus.
3. **`encode(x, profil)` byte-identisch über zwei unabhängige
   Implementierungen**, über den gesamten Vektorsatz.
   **Erbracht, mit einer benannten Lücke.** `rust/` und
   `conformance/reference.py`, die zweite aus diesem Dokument geschrieben und
   ohne eine Zeile gemeinsamen Code. Sie stimmen über alle 308 Vektorpaare
   überein, über fünfzehn Fehlerfälle, und über eine 262923-Byte-Eingabe
   Zeichen für Zeichen (`conformance/test_large.py`). Die Lücke: derselbe
   Autor.

Ergänzende Arbeiten, nicht normativ:

4. Messen (§12, §13): Korpusdichte und Durchsatz über binary2textbench —
   **erbracht**, die Zahlen stehen dort.
5. Container-Test mit echten Parsern — **erledigt für Pythons Parser**,
   `conformance/test_containers.py`. Profil U geht durch URL, Cookie, JSON,
   Dateiname und Logzeile unverändert; Profil T braucht in einer URL
   Prozent-Encoding und enthält das Leerzeichen.
6. API-Form je Zielsprache: `encode` / `decode` analog zum dortigen `base64`;
   zusätzlich `decode_url_strict` und `encode_base64url`, und sonst nichts.
   Rust liegt bei; `python/` ist ein PyO3-Binding darüber. Ein Binding ist
   ausdrücklich **keine** zweite Implementierung im Sinne von Nachweis 3.
7. Vektorsatz: `docs/vectors.json` führt 183 Vektoren. Sie decken die
   Blockgrenze bei 48, den Gleichstand bei 27 legalen Bytes von beiden Enden
   des Blocks, und die Tails von 4, 5 und 6 Bytes ab.

## 17. Erweiterungskandidaten (nicht Teil von v0.4)

1. **Wahlfreier Zugriff.** Blockgrenzen liegen an festen Eingabe-Offsets,
   aber an variablen Ausgabe-Offsets. Ein Index der Blockanfänge, außerhalb
   des Stroms, gibt O(1)-Zugriff; ein Format, das ihn im Strom trägt, wäre
   eine eigene Frage.
2. **Profil-Aushandlung.** Aus dem Strom prinzipiell nicht ableitbar (§7.2).
3. **Eine vektorisierte Maskenverarbeitung.** Kein Formatthema, aber die
   Stelle, an der die Implementierung steht: der Maskenblock kostet das
   Dreifache von Base64, weil das Trennen und Zusammenfügen der Bytes ohne
   SIMD acht Lade-Speicher-Paare je Gruppe braucht (§13.3).
4. **Eine andere Blockgröße.** 48 ist begründet (§4), nicht bewiesen. Wer
   eine Zahl ändern will, ändert die Versionsnummer.
