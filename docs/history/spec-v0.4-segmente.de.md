# Base65t — Spezifikation v0.4 (final)

**Status:** final. Ersetzt v0.2 und v0.1; beide liegen mit ihrer Errata und den
Messprotokollen in `docs/history/`, zusammen mit einem Verzeichnis dessen, was
zwischen den Fassungen gestrichen wurde und warum.
**Kurzfassung:** Base64URL, erweitert um ein 65. Zeichen (`~`), das
längenpräfigierte Klartext-Segmente einleitet.

**Am Wire-Format ändert v0.4 nichts, was v0.1 nicht schon konnte.** Jeder
v0.4-Strom ist ein v0.1-Strom. Was v0.4 streicht, sind Wahlmöglichkeiten:
fünf Presets, ein Framed Mode, ein drittes Profil. Ein Dekoder, der v0.1
gelesen hat, liest v0.4; ein v0.4-Dekoder lehnt gerahmte Ströme ab, weil es
sie nicht mehr gibt.

> Normative Aussagen sind als solche gekennzeichnet und verwenden MUSS / DARF NICHT /
> SOLLTE nach RFC 2119. Zahlen, die nicht als *exakt* markiert sind, sind Messungen
> auf dem in §16.5 genannten Korpus oder Schätzungen; letztere sind mit **[OFFEN]**
> gekennzeichnet, nicht normativ, und ihre spätere Bestimmung ändert das Format nicht.

## Änderungen gegenüber v0.2

Die Abschnittsnummern sind unverändert, damit jede Referenz aus v0.1 und v0.2
weiter trägt. Wo ein Abschnitt entfällt, steht seine Nummer mit einer Zeile da,
die sagt, was dort stand und wo es hin ist.

| § | Änderung |
|---|---|
| 0.1 | **Keine Presets mehr.** Ein Encoder, eine parameterlose `encode`-Funktion. Die Begründung steht dort und ist die wichtigste Änderung dieser Fassung |
| 5.5 | `framing_seen` entfällt; das Ergebnis trägt zwei Felder statt drei |
| 5.6 | **Zurückgezogen** (Framing-Erkennung, Regel F) |
| 6.1 | `L1 = 0` bleibt reserviert, trägt aber nichts mehr — die zwei Zeichen `~A` sind für eine spätere Revision frei |
| 7 | **Profil B entfällt.** Zwei Profile, beide druckbares ASCII |
| 8 | **Zurückgezogen** (Framed Mode, F1/F2/F′) |
| 9.2.1 | War die lineare Regel; ist jetzt die **Fensterung**, die dem exakten Programm konstanten Speicher gibt |
| 9.2.1.1 | Zurückgezogen (Parallelisierung als Formataussage) |
| 9.2.1.2 | Bleibt: die Maske, jetzt für den Klartext-Test von §9.2.4 |
| 9.2.2 | War „warum `dense` nicht das Programm benutzt"; ist jetzt die Gegenrechnung dazu, mit Zahlen |
| 9.2.4 | **Neu:** die geschlossene Form, die das Programm für reinen Klartext ersetzt, ohne ein Byte zu ändern |
| 9.3 | War die Preset-Tabelle; ist jetzt die Liste der zwei Einstiegspunkte |
| 9.4 | Gilt ohne Ausnahme, weil die Ausnahme (`framed`) gestrichen ist |
| 9.5 | Als Formatfrage geschlossen; `L_min` ist 1 und §9.1 begründet nur noch, warum §9.4 auch ohne Optimierung hielte |
| 9.6 | War `dense-fast` mit einer Stichprobe je Fenster; ist jetzt **eine** Entscheidung am Dateikopf, aus Magic Number und Entropie |
| 10.2, 10.3 | Ein Einstiegspunkt statt drei; der Framed-Zweig entfällt |
| 10.4 | Zehn Fehlercodes statt zwölf |
| 11, 11.1 | Der Encoder ist eine Funktion **einer** Eingabe und eines Profils. `encode_canonical` ist keine eigene Funktion mehr, weil `encode` sie ist |
| 12 | Die Profil-B-Zeile entfällt |
| 13 | Vollständig neu gemessen. Die Zahlen sind teilweise deutlich schlechter als in v0.2, und §13.3 sagt, warum und was es kostet |
| 15 | Dreizehn Vektoren; die zu Framing und zum zweiten Encoder sind zurückgezogen, TV13 ist neu |
| 16 | Vier Nachweise; der zum Framed Mode entfällt |
| 17 | Framing ist jetzt ein Erweiterungskandidat statt ein Teil des Formats |

## Was v0.4 zusichert und was nicht

**Größe: zugesichert, je Eingabe, nicht im Mittel.** `len(encode(x)) ≤
ceil(4·len(x)/3)` für jede Eingabe und beide Profile, ohne Ausnahme (§9.4).
Belegt strukturell — die reine Base64-Segmentierung liegt immer in der
Kandidatenmenge — und gemessen über 69 Korpusproben und 202 MiB Silesia.
Schärfer noch: wo nichts zu holen ist, schreibt der Encoder **dieselben Bytes**
wie Base64URL, nicht nur gleich viele.

**Bytegleichheit: zugesichert.** `encode(x, profil)` ist eine Funktion. Kein
Preset, kein Schwellwert, keine Uhr, keine Threadzahl. Zwei Encoder, die diese
Spezifikation umsetzen, schreiben dieselben Bytes, und deshalb darf man die
Ausgabe cachen, vergleichen und als Cache-Key verwenden. Signieren darf man
sie trotzdem nicht — §11 sagt, warum das eine Aussage über den *Dekoder* ist.

**Durchsatz: nicht zugesichert, und v0.4 ist an einer Stelle deutlich
langsamer als v0.2.** Wo der Klartext-Test aus §9.2.4 greift — die kurzen
Werte, für die das Format gedacht ist — kodiert Base65t in 37 bis 90 % der
Zeit von Base64. Wo er nicht greift, läuft das Programm aus §9.2, und das
kostet das Sechs- bis Elffache. §13.3 beziffert beides und benennt es als
offenen Punkt statt es zu verschweigen.

## 0. Positionierung (nicht normativ)

### 0.1 Ein Format, ein Encoder

v0.1 hatte fünf Presets, v0.2 sechs. v0.4 hat keine, und das ist die
Entscheidung, aus der alle anderen dieser Fassung folgen.

Der Grund ist nicht Sparsamkeit, sondern wofür dieses Format da ist. Es gibt
dichtere Verfahren (Base85N, Base91z, Z85), und wer Dichte will, nimmt eines
davon. Base65t ist für den Fall gedacht, in dem jemand **unsicher** ist: er hat
etwas, das ohnehin schon Text ist, und muss es durch einen Kanal schicken, der
Bytes akzeptieren muss. Base64 kann er, Base64 versteht jeder, Base64 ist
niemals falsch. Alles, was er über ein anderes Verfahren erst lernen muss,
ist ein Grund, es bleiben zu lassen.

Ein Preset ist genau so ein Grund. Wer zwischen einem dichten, einem
kanonischen und einem schnellen Encoder wählen muss, muss wissen, was diese
Wörter bedeuten, bevor er ein Byte kodieren kann — und wer unsicher ist,
greift zu Base64.

> **Deshalb: eine parameterlose `encode`-Funktion, die Bytes nimmt und Bytes
> liefert.** Was der Encoder tut, entscheidet er selbst (§9.6).

Zwei Parameter bleiben, und keiner davon ist eine Wahl über die Kodierung:

* **Das Profil** (§7) ist eine Aussage über den *Container*, nicht über den
  Strom, und aus dem Strom nicht ableitbar (§7.2). Der Default ist U.
* **`encode_base64url`** (§14) ist kein Modus des Formats, sondern der Ausgang
  aus ihm: für einen Aufrufer, der ein Geheimnis trägt und nichts davon im
  Klartext stehen haben will, und für einen, der nur Base64URL sprechen darf
  und diese Bibliothek als einzige Abhängigkeit möchte.

| Einsatz | Profil | worauf es ankommt |
|---------|--------|-------------------|
| URL-Query | U | URL-Sicherheit ohne Prozent-Encoding |
| Cookie-Wert | U | `cookie-octet`-Konformität (§7.1) |
| HTTP-Header | U oder T | ASCII, keine Trennzeichen |
| Cache-/Dedup-Key | wie der Container | Bytegleichheit (§11.1), kürzeste Ausgabe |
| Log-Feld | T | dort bleibt der Text lesbar (§13.4) |
| Token mit Geheimnisanteil | — | `encode_base64url`, keine Klartext-Leaks (§14) |

### 0.2 Was Base65t *ist*, in einem Satz

> Ein segmentiertes Hybrid-Encoding: Base64URL als Binärrepräsentation,
> längenbegrenzte Rohbyte-Literale als zweite Repräsentation, und ein 65. Zeichen
> als Diskriminator zwischen beiden.

### 0.3 Was „ein Dekoder für alles" genau heißt

> Ein konformer `decode()` nimmt einen Octet-Stream und ein Profil entgegen und
> benötigt **keinen weiteren Parameter**. Alphabetvariante (§5.2) und Padding
> (§5.3) werden aus dem Strom selbst bestimmt und im Ergebnis gemeldet.

Das Profil bleibt Parameter, weil es eine Aussage über den *Container* ist, nicht
über den Strom, und aus dem Strom prinzipiell nicht ableitbar (§7.2).

### 0.4 Warum nicht „Base85N mit URL-Alphabet"

Die *unreserved*-Menge von RFC 3986 umfasst 66 Zeichen. Eine Radix-85-Kodierung ist
darin darstellbar, aber ein Passthrough vom Typ Base85N braucht zusätzlich
Spender-Zeichen für die R-Set-Substitution — dafür bleibt kein Spielraum, sobald 64
Zeichen für den Binärkern gebunden sind. Base65t geht den entgegengesetzten Weg: ein
Kern, der **exakt Base64URL ist**, plus ein Diskriminator. Nur daraus folgt die
Superset-Eigenschaft aus §5.2; mit einem Radix-85-Kern wäre sie nicht zu haben.

### 0.5 Wo Base65t in der Familie steht (nicht normativ)

Base65t ist der **Wegbereiter**, nicht der Höhepunkt. Base85N führt denselben
Segmentgedanken weiter und ist dichter; Base91z komprimiert. Beide verlangen
vom Aufrufer, ein Verfahren zu lernen, das er noch nicht kennt, und zu
beurteilen, ob sein Container es trägt.

Base65t verlangt das nicht: der Kern *ist* Base64URL, die Ausgabe ist niemals
größer, und die einzige Frage, die überhaupt zu beantworten bleibt, ist das
Profil. Der Mehrwert ist entsprechend klein — über den Korpus 1,5 %, auf
kurzen Werten 22 % — aber die Kosten sind es auch. Wer damit gelernt hat, was
ein längenpräfigiertes Literal in einem Textstrom leistet, kann zu Base85N
weitergehen; wer Kompression braucht, zu Base91z. Diese Fassung ist dafür
gemacht, dass der erste Schritt keine Entscheidung erfordert.

## 1. Zielsetzung

1. **Binär nie schlechter als Base64** (§9.4).
2. Profil-legalen Klartext nahezu verlustfrei durchreichen (≈ 1,001).
3. Lesbar bleiben.
4. **Kein Escaping** — auch nicht für `~`.
5. **Abwärtskompatibel lesen** — jeder kanonische Base64- oder Base64URL-Strom,
   gepaddet oder nicht, dekodiert zu denselben Bytes (§5.2, §5.3). Normativ.
6. **Selbstbestimmend im Strom** — Alphabet und Padding werden erkannt,
   nicht konfiguriert (§0.3).
7. Bytegleich reproduzierbar (§11.1).

### 1.1 Die Kompatibilität ist asymmetrisch

| Richtung | Gilt? |
|----------|-------|
| Base65t-Dekoder liest Base64URL, ungepaddet | **ja**, normativ |
| Base65t-Dekoder liest Base64URL, gepaddet | **ja**, normativ, §5.3 |
| Base65t-Dekoder liest klassisches Base64 (`+`/`/`), gepaddet oder nicht | **ja**, normativ, §5.2/§5.3 |
| **Base64-Dekoder liest Base65t** | **nein** — `~` ist nicht im Alphabet |

Base65t ist ein *Superset auf der Leseseite*. Migrationspfad: erst Dekoder ausrollen,
später Encoder umstellen. Kein Flag Day.

Umgekehrt: wo ein fremder Base64-Dekoder am anderen Ende sitzt, bleibt Base64 die
richtige Antwort. Das lässt sich nicht wegformulieren.

**Kanonizität der Eingabe.** Die Aussage gilt für *kanonische* Ströme. Ein
Base64-Strom mit gesetzten Restbits (z. B. `YWxpY2V` mit Müll im letzten Quantum)
wird mit `E_NONZERO_TAIL` abgewiesen — auch dann, wenn eine permissive
Base64-Bibliothek ihn akzeptiert hätte. Das ist Absicht und gehört in den
Differential-Fuzzing-Korpus (§16.2) als *erwartete Abweichung*, nicht als Bug.

## 2. Nicht-Ziele

* **Kein Kompressionsformat.** Ab ca. 1 KB Text schlägt `gzip` + Base64 deutlich.
* **Kein Dichte-Rekord.** Z85 (1,25), basE91 (1,23), Base85N sind binär dichter.
* **Kein Durchsatz-Rekord.** Durchsatz ist ein *Ziel* (§13.2) — Base64 ist
  dabei der Maßstab, nicht die SIMD-Bestenliste, und Dichte wird nie gegen ihn
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
Wert 0–63 interpretiert: genau die Zeichen von Base64-Segmenten und von
Längen-Headern — **nicht** die Bytes einer Literal-Payload. Tragend für §5.4.

## 4. Streamstruktur

```
Stream  := Segment*
Segment := Base64Segment | LiteralSegment
```

Segmente sind unabhängig: **ein Base64-Quantum überschreitet niemals eine
Segmentgrenze.** Bindend für §13.1.

**Base64-Läufe sind maximal (normativ).** Zwischen zwei Base64-Segmenten steht kein
Trennzeichen; zwei aneinandergrenzende Base64-Segmente sind vom Dekoder nicht
unterscheidbar und dekodieren zu anderen Bytes als vom Encoder gemeint. Ein Encoder
DARF einen Base64-Lauf deshalb **nicht** aufteilen. Das ist auch die Bedingung, an
der die Fensterung aus §9.2.1 hängt.

**Literal-Läufe sind dagegen *nicht* automatisch maximal.** Zwei angrenzende
LiteralSegmente tragen je einen eigenen Header und sind vom Dekoder sehr wohl
unterscheidbar; die Grammatik erlaubt sie ausdrücklich. Für den Encoder ist das eine
echte Wahl — und der Grund, warum eine bloße Byte-Klassifikation für §11.1 nicht
ausreicht.

Ein leerer Strom ist gültig und dekodiert zu null Bytes.

## 5. Base64-Segment

Base64URL. Sei `n` die Zeichenzahl ohne Padding:

| `n mod 4` | Bytes im letzten Quantum | Gültig |
|-----------|--------------------------|--------|
| 0 | 3 | ja |
| 2 | 1 | ja |
| 3 | 2 | ja |
| 1 | — | **nein** (`E_ALIGN`) |

**Kanonizität:** ungenutzte Bits im letzten Zeichen MÜSSEN 0 sein
(`E_NONZERO_TAIL`).

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
Gilt für Base64-Segmentzeichen **und** Längen-Header.

### 5.3 Padding (normativ)

RFC 4648 §3.2 macht Padding zur Pflicht, sofern die aufrufende Spezifikation nichts
anderes bestimmt. Klassisches Base64 ist in der Praxis **immer** gepaddet.

> **Regel P.** Ein Base64-Segment, das **am Stromende** endet, DARF mit 1 oder 2 `=`
> abgeschlossen sein. Der Dekoder MUSS diese akzeptieren. „Strom" heißt der
> ganze Oktett-Strom.

| `k` (`=`-Anzahl) | erforderlich |
|------------------|--------------|
| 0 | `n mod 4 ∈ {0, 2, 3}` |
| 1 | `n mod 4 == 3` |
| 2 | `n mod 4 == 2` |

Jede andere Kombination → `E_PADDING`. Padding ist durch `n mod 4` determiniert,
also **prüfbar redundant**, nicht mehrdeutig.

`=` an jeder anderen Position → `E_CHARSET`. Insbesondere darf ein Segment, auf das
`~` folgt, nicht gepaddet sein — sonst wäre `=` ein zweiter Segment-Terminator und
müsste in die Vektor-Schleife (§13.1).

**Implementierungsfalle.** Padding DARF NICHT vorab vom Stromende gestrippt werden.
In Profil T ist `=` (0x3D) legales Literal-Byte; `~Ea=b=` würde verstümmelt.
Behandlung erfolgt beim Scannen des Segments (§10.1, TV10).

### 5.4 Alphabet-Konsistenz (Regel A, normativ)

Ohne Zusatzregel hätte ein Strom mit `k` Alphabetzeichen aus {62, 63} genau `2^k`
Schreibweisen derselben Bytes.

> **Regel A.** Ein Strom DARF NICHT beide Alphabetvarianten mischen. Enthält die
> Menge der **Alphabetzeichen** sowohl ein Zeichen aus {`+`,`/`} als auch eines aus
> {`-`,`_`} → `E_MIXED_ALPHABET`. Der Strom ist dabei der ganze Oktett-Strom.

Damit sinkt die Ambiguität von `2^k` auf **2**. Kosten: ein Bit Dekoderzustand.

**Wichtig:** Regel A betrifft ausschließlich Alphabetzeichen. Literal-Payloads
zählen nicht mit — in Profil U enthält fast jede Payload `-` oder `_`. Wer den
Gesamtstrom scannt, weist gültige Ströme ab (TV7).

Regel A ist außerdem der Grund, warum ein vektorisierter Dekoder überhaupt
möglich ist: sie fragt nur, *ob* ein Lauf ein Zeichen aus einer der beiden
Mengen enthält, und das ist eine **Suche und keine Dekodierung** (§13.1.1).

### 5.5 Meldung und strikte Varianten (normativ)

Permissivität darf die Validierung nicht kosten. Ein `decode()`-Ergebnis MUSS
enthalten:

```
alphabet_seen : { none, url, classic }
padding_seen  : bool
```

Zusätzlich MUSS `decode_url_strict` angeboten werden (weist `classic` mit
`E_NON_URL_ALPHABET` ab). Beide Flags fallen im Parser ohnehin an; die Meldung
ist zur Laufzeit gratis.

### 5.6 Framing-Erkennung — **zurückgezogen**

Hier stand Regel F: ein Strom, dessen erste zwei Oktette `~A` sind, ist gerahmt.
Mit dem Framed Mode (§8) ist sie entfallen. Ein Strom, der mit `~A` beginnt, ist
in v0.4 kein gerahmter Strom, sondern ein ungültiger: `E_RESERVED_LEN` (§6.1).

Der Verzicht ist die dritte Auto-Erkennung, die §14 nicht mehr als
Parser-Differential-Fläche aufzählen muss.

## 6. Literal-Segment

```
LiteralSegment := "~" LengthHeader Payload
```

### 6.1 Längen-Header

| `L1` | Bedeutung |
|------|-----------|
| 0 (`A`) | **Reserviert.** `E_RESERVED_LEN`. Trägt in v0.4 nichts; die zwei Zeichen `~A` stehen einer späteren Revision zur Verfügung (§17) |
| 1–62 | Länge = `L1`. Header 2 chars. |
| 63 (`_`/`/`) | Erweiterung: nächste zwei Zeichen = 12 Bit `V`. Länge = `63 + V`, Bereich 63–4158. Header 4 chars. |

Encoder MUSS die kürzeste Header-Form wählen. Läufe > 4158 Bytes → mehrere
LiteralSegmente. Dichte reinen Literals: `4162/4158 = 1,00096`.

Die **zwei Bänder** — `h(m) = 2` für `m ≤ 62`, `h(m) = 4` für `63 ≤ m ≤ 4158` —
sind keine Kosmetik: an ihnen hängt die Kantenmenge des Programms in §9.2 und
die Fallunterscheidung der geschlossenen Form in §9.2.4.

### 6.2 Payload

Exakt `L` rohe Bytes, unverändert. Kein Zeichen der Payload ist Steuerzeichen — auch
`~` nicht. Kein Escaping, weil es nichts zu escapen gibt.

## 7. Profile

| Profil | Erlaubte Literal-Bytes | URL-Query direkt? |
|--------|------------------------|-------------------|
| **U** (Default) | RFC-3986-*unreserved* (66 Zeichen) | **ja** |
| **T** | ASCII 0x20–0x7E ohne `"` und `\` (93 Zeichen) | nein |

Profilwidrige Payload → `E_PROFILE`. Ein profilwidriges Byte ist kein Sonderfall: es
landet im Base64-Segment.

**Profil T** ist JSON-String-sicher, **nicht** CSV-struktursicher und **nicht**
URL-sicher: `,` `;` `?` `&` `=` `+` `/` `#` sind erlaubt. **Und es enthält das
Leerzeichen** (0x20): eine whitespace-getrennte Logzeile muss einen T-Wert also
quoten, ein `key=value`-Format nicht. Wer eine Logzeile an Leerzeichen
zerlegt, nimmt Profil U — dessen Alphabet enthält keines. Gefunden vom
Container-Test aus §16.6, nicht aus der Tabelle abgelesen.

**Profil B ist gestrichen.** Es erlaubte jedes Oktett im Literal und verließ
damit die ASCII-Eigenschaft. „Die Ausgabe ist Text" ist der Satz, wegen dem
jemand dieses Format überhaupt anschaut; ein Profil, das ihn mit einer Fußnote
versieht, kostet mehr als es bringt.

### 7.1 Cookie-Konformität von Profil U (bewiesen, nicht gemessen)

RFC 6265 §4.1.1 definiert:

```
cookie-octet = %x21 / %x23-2B / %x2D-3A / %x3C-5B / %x5D-7E
```

Das Alphabet von Profil U — 62 Alphanumerische plus `-` (0x2D), `.` (0x2E),
`_` (0x5F), `~` (0x7E) — liegt vollständig in diesen Bereichen:

| Zeichenklasse | Bereich | cookie-octet-Bereich |
|---------------|---------|----------------------|
| `0`–`9` | 0x30–0x39 | %x2D-3A ✓ |
| `A`–`Z` | 0x41–0x5A | %x3C-5B ✓ |
| `a`–`z` | 0x61–0x7A | %x5D-7E ✓ |
| `-` `.` | 0x2D, 0x2E | %x2D-3A ✓ |
| `_` `~` | 0x5F, 0x7E | %x5D-7E ✓ |

Alle 66 Zeichen geprüft, keine Ausnahme. Die Aussage folgt aus der ABNF und ist
damit **beweisbar, nicht empirisch**.

**Teilweise beantwortet, davon getrennt** bleibt die schwächere, empirische
Frage, ob reale Cookie-Parser sich an die ABNF halten. Pythons `http.cookies`
tut es (§16.6): ein Profil-U-Wert wird weder gequotet noch verändert. Browser,
Proxies und Frameworks sind damit **nicht** geprüft, und keine dieser Messungen
ist ein Beleg für die Aussage oben — die folgt aus der ABNF und braucht keinen.

### 7.2 Warum das Profil Parameter bleibt

Das Profil ist aus dem Strom nicht ableitbar: ein Strom, dessen Literale zufällig nur
*unreserved*-Bytes enthalten, ist unter U und T identisch gültig. Es beschreibt
die Erwartung des **Containers**, nicht eine Eigenschaft des Stroms. Deshalb ist es
der einzige Parameter, den `decode()` behält (§0.3).

## 8. Framed Mode — **zurückgezogen**

v0.1 und v0.2 hatten hier einen zweiten Modus: Frames à 65536 Bytes mit einem
`~A`-Marker und einer 18-Bit-Länge, für wahlfreien Zugriff, dazu die Invariante
F′ und die encoderseitigen Regeln F1 und F2.

Er ist gestrichen, und der Grund steht in §9.4: `framed` war die einzige
Ausnahme von der Nie-schlechter-Garantie. Eine Garantie mit Ausnahme ist eine
Garantie, die niemand zitiert — und die Ausnahme wurde für einen Zufallszugriff
bezahlt, nach dem niemand gefragt hatte. Mit ihr entfallen die Regeln F1/F2/F′,
die Fehlercodes `E_FRAME_RULE` und `E_FRAME_SYNC`, die Framing-Erkennung (§5.6)
und die Testvektoren TV5b, TV9a, TV9b, TV11 und TV15 der v0.2.

Was bleibt: `L1 = 0` ist weiter reserviert (§6.1). Eine spätere Revision kann
die zwei Zeichen `~A` erneut belegen, und §17 nennt Framing als Kandidaten.

## 9. Encoder

### 9.0 Grundprinzip (normativ)

> **Der Encoder ist eine Funktion.** `encode(x, profil)` MUSS durch die Eingabe
> und das Profil eindeutig bestimmt sein — nicht durch eine Uhr, eine
> Threadzahl, eine Maschineneigenschaft oder eine Aufrufreihenfolge.

Er optimiert über die Menge der gültigen Segmentierungen (§4, §7) auf die
kürzeste Ausgabe, und weil mehrere Segmentierungen gleich lang sein können:

> Haben mehrere gültige Segmentierungen dieselbe Länge, MUSS ein Encoder die
> nach der Ordnung aus §11.1 kleinste wählen.

Damit prüft ein Testvektor Bytes statt Längen — was §16.8 braucht und was
`docs/vectors.json` über 137 Vektoren tut.

**Eine Ausnahme von „kürzeste Ausgabe" gibt es**, und sie ist in §9.6
beschrieben: wo eine Prüfung am Dateikopf sagt, dass nichts zu finden ist,
schreibt der Encoder Base64URL, ohne zu suchen. Die Entscheidung ist selbst
eine Funktion der Eingabe, §9.0 bleibt also unberührt; was sie kosten kann,
ist Größe, nie die Garantie aus §9.4 (§9.6).

### 9.1 Schwellwert — was er einmal war und was er noch begründet

Ein Literal spart `L/3` chars, kostet 2 chars Header plus Rundungsverschnitt. Mit
`r(B) = ceil(4B/3) − 4B/3 ∈ {0, ⅓, ⅔}` und maximaler Zusatzrundung `4/3`:

```
Ersparnis_worst(L) = L/3 − 2 − 4/3 = (L − 10)/3
```

`L ≤ 9` negativ, `L = 10` neutral, `L ≥ 11` immer ein Gewinn.

**In v0.4 ist das kein Encoder-Parameter mehr.** `L_min` ist 1: der Encoder
nimmt ein Literal, wo immer es kürzer ist, bei günstiger Ausrichtung bis hinab
zu **sieben Bytes** (9 statt 10 chars). `Ersparnis_worst(10) = 0` ist der
*Worst Case*; im besten Fall spart dasselbe Literal 2 chars, und ein Encoder,
der minimiert, findet diesen Fall.

Was die Rechnung weiter trägt, ist eine Aussage über *jeden denkbaren*
Encoder: sie lädt einem einzelnen Literal bereits die schlechteste Rundung auf
beiden Seiten auf. Für `k ≥ 1` Literale mit Header `h_j ∈ {2, 4}` und Längen
`L_j ≥ 11` gilt gegenüber reinem Base64

```
Differenz  ≤  Σ_j (h_j − L_j/3)  +  2(k+1)/3  ≤  −k + 2/3  <  0
```

Der zweite Term ist die Rundung der höchstens `k + 1` Base64-Läufe, der erste
je Literal höchstens `2 − 11/3 = −5/3`. **Wer §9.4 halten will, ohne zu
optimieren, kann das** — er nimmt nur Literale ab elf Bytes. v0.2 hat davon
Gebrauch gemacht; v0.4 optimiert und bekommt die Garantie ohnehin (§9.4).

### 9.2 Optimale Segmentierung — Herleitung

**Literale werden nicht als Zustand, sondern als Kante modelliert.**

**Definitionen.** `D[j]` = minimale Kosten, die Bytes `[0, j)` so zu kodieren, dass
bei `j` eine Segmentgrenze liegt. `B[j][p]` = minimale Kosten für `[0, j)` mit
offenem Base64-Segment, `p ∈ {0,1,2}` Bytes im angebrochenen Quantum.

**Base64-Kanten** sind O(1): die Zeichenkosten je Byte hängen nur von `p` ab
(`p=0→1`: +2 chars, `p=1→2`: +1, `p=2→0`: +1; Summe 4 chars je 3 Bytes ✓). Ein
Base64-Segment darf bei jedem `p` enden, also `D[j] ← min_p B[j][p]`.

**Literal-Kanten.** Ein Literal von `i` nach `j` mit `m = j − i` kostet
`m + h(m)` mit `h` aus §6.1. Läufe > 4158 entstehen als **mehrere** Kanten,
brauchen also keinen dritten Fall. Damit hat `h` genau zwei Bänder, und:

```
D[j] ← min(  j + 2 + min{ D[i] − i : j−62   ≤ i ≤ j−1  },
             j + 4 + min{ D[i] − i : j−4158 ≤ i ≤ j−63 }  )
```

Beide inneren Terme sind **Schiebefenster-Minima fester Breite** über der Folge
`D[i] − i`. Mit je einer monotonen Deque sind sie **O(1) amortisiert** pro Position.
Damit ist der DP insgesamt **O(n) Zeit**.

**Zulässigkeit der Kanten.** Hier ist Sorgfalt nötig: es gibt O(n·4158) potentielle
Kanten, eine Prüfung *pro Kante* würde den O(n)-Beweis zerstören. Jede
Zulässigkeitsbedingung muss sich als Einschränkung des gültigen `i`-Fensters
ausdrücken lassen, damit die Deques sie tragen. In v0.4 gibt es nur noch eine:

* **Profil.** Eine Literal-Kante `[i, j)` verlangt durchgehend profil-legale Bytes.
  Sei `bad(j)` die letzte profilwidrige Position vor `j`; gültiges Fenster ist
  `i > bad(j)`. Umgesetzt durch Leeren beider Deques beim Passieren eines
  profilwidrigen Bytes — O(1) amortisiert.

Die Bedingungen F1 und F2, die v0.2 hier zusätzlich führen musste, sind mit dem
Framed Mode entfallen (§8).

**Rechnung in Dritteln.** Ein Base64-Quantum kostet vier Zeichen für drei Bytes,
also ist eine Kostenfunktion je Byte nur in Dritteln ganzzahlig. Eine
Implementierung MUSS in Dritteln rechnen oder auf andere Weise exakt bleiben:
das Minimum wird auf Gleichheit geprüft (§11.1), und eine Rundung an dieser
Stelle macht aus einer Ordnung eine Meinung.

**Speicher.** Kostenberechnung O(1) zusätzlich (zwei Deques der Breite 62 bzw. 4096
→ konstant). Die **Tabelle** ist O(n) — siehe §9.2.1.

### 9.2.1 Fensterung (normativ)

Das Programm braucht eine Kostentabelle über der Eingabe, und die ist O(n): rund
zwanzig Byte je Eingabebyte. Für ein Gigabyte-Objekt ist das der Unterschied
zwischen „läuft" und „läuft nicht".

> **Regel.** Die Eingabe wird in **Fenster von 65536 Bytes** geschnitten, an
> absoluten Offsets ab Eingabeanfang. Das Programm aus §9.2 läuft je Fenster.
> Endet ein Fenster mit einem Base64-Segment und beginnt das nächste mit einem,
> so sind das **zusammen ein Segment** (§4) und MÜSSEN als eines geschrieben
> werden.

Der zweite Satz ist keine Optimierung, sondern die Korrektheitsbedingung. Ein
Base64-Lauf von `k` Bytes mit `k mod 3 ≠ 0` lässt ein Quantum offen; wird die
Naht zweimal geschrieben statt einmal, dekodiert sie zu etwas, das keines der
beiden Fenster gemeint hat. Genau dieser Fehler ist in der Bench aufgetreten
und nicht in den Testvektoren — die reichen nicht über ein Fenster hinaus.

**Was die Fensterung kostet.** Ein Literal kann keine Fenstergrenze
überspannen, also höchstens ein zusätzlicher Header je Grenze: **≤ 4 Zeichen je
65536 Bytes, unter 0,01 %**. Gemessen liegt es darunter, weil an einer
zufälligen Grenze meist ohnehin ein Segmentwechsel liegt.
`windowing_costs_almost_nothing` in der Testsuite misst es gegen dasselbe
Programm über die ganze Eingabe.

**Warum die Größe keine Rolle spielt, ob sie durch 3 teilbar ist.** v0.2
brauchte für die Blockbildung 65535 = 3·21845, weil dort jeder Block *einzeln*
zu Base64 gerundet wurde. Hier wird nicht geschnitten, sondern nur getrennt
gerechnet und wieder zusammengefügt; die Naht ist ein Segment, und die Rundung
findet einmal am Segmentende statt. 65536 ist deshalb frei wählbar und wurde
gewählt, weil eine Zweierpotenz die Offsetrechnung trivial macht.

### 9.2.1.2 Die Maske (nicht normativ)

Die Frage „wo endet der profil-legale Lauf" ist ein datenabhängiger Sprung, den
kein Sprungvorhersager erraten kann, und ein Encoder, der sie je Byte stellt,
bezahlt auf Fließtext eine Fehlvorhersage je Lauf.

Sie lässt sich für 64 Bytes auf einmal beantworten: eine Tabellensuche je Byte,
das Ergebnis als ein Bit, sechzig­vier Bits in ein Wort. Gemessen 473 MiB/s für
den byteweisen Scan gegen 1352 MiB/s für die Maske — und die 1352 hängen nicht
von den Daten ab. Die Maske ist auch, was den Klartext-Test aus §9.2.4 billig
macht.

Ein Detail, das mehr ausmacht als es sollte: die naheliegende Schleife
`m |= bit << k` über alle vierundsechzig ist **eine** Abhängigkeitskette der
Länge 64 und läuft mit 926 MiB/s. Acht Ketten zu acht, jede mit konstanter
Schiebeweite, laufen mit 1352.

### 9.2.2 Was das exakte Programm kostet

v0.2 definierte den Default-Encoder durch eine lineare Regel und nicht durch
das Programm aus §9.2, mit der Begründung, das Programm sei zu langsam. Die
Begründung war richtig; die Zahl dahinter war es in v0.2 nicht. Gemessen ist
das Programm **21- bis 63-mal** langsamer als die lineare Regel, nicht zwölfmal
wie dort behauptet, und es gewinnt dafür 0,22 % Größe über den Korpus.

v0.4 nimmt das Programm trotzdem als einzigen Encoder, aus drei Gründen:

1. **Es gibt kein zweites Preset mehr, gegen das man tauschen könnte** (§0.1).
   Eine Regel, die nicht längenoptimal ist, kann §11.1 nicht erfüllen, und
   §11.1 ist die Bytegleichheit, an der Cache-Keys hängen.
2. **Wo es teuer ist, wird es gar nicht ausgeführt** (§9.6): auf komprimierten
   und hochentropen Daten entscheidet der Dateikopf, und der Encoder schreibt
   Base64URL, ohne zu suchen.
3. **Wo das Format seinen Wert hat, ist die Antwort ohne Programm bekannt**
   (§9.2.4).

Was nach diesen dreien übrig bleibt — Eingaben, die weder hochentropisch noch
durchgehend profil-legal sind, also gemischter Text — kostet das Sechs- bis
Elffache der Base64-Zeit für 0 bis 1,5 % Größe. §13.3 beziffert es und nennt
es einen offenen Punkt.

### 9.2.4 Die geschlossene Form für reinen Klartext (nicht normativ)

Ist **jedes** Byte der Eingabe profil-legal und passt die Eingabe in ein
Literal-Segment (`n ≤ 4158`), so lässt sich die Antwort des Programms
hinschreiben, statt sie zu rechnen. Das ist keine zweite Regel und keine
Näherung: es ist dieselbe Segmentierung, nur anders hergeleitet.

**Herleitung.** Zwei angrenzende Base64-Läufe sind ein Lauf (§4), und `ceil`
ist über eine Aufteilung superadditiv — ein Optimum braucht also höchstens
*einen* Base64-Lauf, und `B` als kleinstes Symbol (§11.1) stellt ihn nach vorn.
Mit `B` Base64-Bytes und dem Rest in Literalen ist der Aufschlag über `n`

```
extra(B) = ceil(B/3) + cover(n − B),   cover(M) = min(2·ceil(M/62), 4)
```

weil ein Literal bis 62 Bytes zwei Header-Zeichen trägt und eines bis 4158 vier
(§6.1). Minimiert, mit Gleichständen nach `B < L < S` ab Index 0 aufgelöst:

| `n` | Segmentierung | warum |
|---|---|---|
| 1–6 | Base64 | `ceil(n/3) ≤ 2`; bei 4, 5, 6 Gleichstand, den `B` gewinnt |
| 7–62 | ein Literal | `n + 2`, und jedes Base64-Byte kostet mindestens eines und spart keines |
| 63–65 | 3 Base64-Bytes, dann ein Literal | die drei kosten ein Zeichen über ihre Länge und holen das Literal aus dem Vier-Zeichen-Band ins Zwei-Zeichen-Band, was zwei wert ist. Drei statt eins oder zwei, weil alle drei gleich lang sind und der längste Base64-Lauf der kleinste Vektor ist |
| 66–68 | 6 Base64-Bytes, dann ein Literal | dasselbe ein Band weiter: sechs kosten zwei und sparen zwei, Gleichstand mit dem einen Vier-Zeichen-Literal, und `B < S` an Index 0 entscheidet für den Lauf |
| 69–4158 | ein Literal | `n + 4`. Unter 125 Gleichstand mit zwei Literalen zu höchstens 62, und `L < S` an der Naht nimmt das eine — der Fall `100 = 50 + 50`, den §11.1 aufwirft |

Oberhalb von 4158 ist die Aufteilungsfrage echt (`4238 + 62` kostet sechs
Header-Zeichen, `4158 + 142` acht), und die geschlossene Form beantwortet sie
nicht; dort läuft das Programm.

**Warum das hier steht, obwohl es nicht normativ ist.** Es ist der Grund, warum
der Encoder auf einer Session-ID, einem Digest, einer UUID und einem JWT
schneller ist als Base64 statt vierzigmal langsamer (§13.4) — und eine zweite
Implementierung, die das Programm ausführt, kommt auf dieselben Bytes. Die
Referenzimplementierung prüft beides gegeneinander, über jede Länge, die die
Form beansprucht, und über beide Profile.

### 9.3 Einstiegspunkte

| Funktion | Was sie tut | Profil |
|---|---|---|
| `encode(x)` | die Kodierung | U |
| `encode(x, profil)` | dieselbe, im Profil, das der Container verlangt | U oder T |
| `encode_base64url(x)` | Base64URL und sonst nichts (§14) | — |

Ein Aufruf ohne Parameter MUSS `encode` + Profil U liefern. Bibliotheken
SOLLTEN genau eine parameterlose `encode`-Funktion exportieren.

`encode_base64url` ist **kein Modus des Formats**: die Ausgabe ist gewöhnliches
ungepaddetes Base64URL, und jeder Base64-Dekoder liest sie. Sie steht hier,
weil zwei Aufrufer sie brauchen — einer trägt ein Geheimnis und will nichts
davon im Klartext (§14), einer spricht mit etwas, das nur Base64URL kann.

### 9.4 Nie-schlechter-Garantie (normativ)

```
len(encode(x, profil)) <= ceil(4 * len(x) / 3)
```

**Je Eingabe, nicht im Mittel, und ohne Ausnahme.** v0.2 nahm `framed` aus;
v0.4 hat die Ausnahme nicht beziffert, sondern gestrichen (§8).

Die Begründung ist strukturell und in beiden Zweigen von §9.6 dieselbe: die
reine Base64-Segmentierung liegt immer in der Kandidatenmenge. Wo das Programm
läuft, minimiert es über diese Menge und kann also nichts Längeres wählen; wo
es nicht läuft, ist die Ausgabe genau dieser Kandidat.

**Schärfer, und der eigentliche Grund für die Umstellung:** auf hochentropen
Daten findet kein Literal einen Platz, und der Encoder schreibt dann nicht nur
gleich viele Zeichen wie Base64URL, sondern **dieselben Bytes**. Wo Base65t
nichts holt, *ist* es Base64.

**Geltungsbereich.** Die Garantie bezieht sich auf die Länge des kodierten Stroms in
Oktetten, nicht auf Transport- oder Container-Overhead. Prozent-Encoding,
Header-Faltung, Cookie-Attribute oder das Framing eines übergeordneten Protokolls
sind nicht eingerechnet.

### 9.5 Segmentwechselrate — als Formatfrage geschlossen

Der Durchsatz hängt an datenabhängigen Verzweigungen, also an Segmentwechseln.

**Was exakt gilt** — eine Aussage über *Segmentierungen*, nicht über Durchsatz. Für
eine Segmentierung, in der jeder Literal-Lauf ≥ `L_min` Bytes und jeder
Base64-Lauf ≥ `B_min` Bytes umfasst (beides **in Bytes**, nicht in chars):

```
Segmentwechsel  ≤  2 pro (L_min + B_min) Eingabebytes
```

Das ist reine Kombinatorik über den Eingabestrom. Ein Durchsatzmodell folgt
daraus **nicht**: die Kosten eines Wechsels hängen an Pipeline-Tiefe,
Sprungvorhersage und Ausgabelänge.

**Für v0.4 ist die Frage geschlossen.** `L_min` ist 1 und `B_min` gibt es
nicht, weil beides Parameter wären und §0.1 keine hat. Eine Messung, die zeigt,
dass ein anderer Wert den Durchsatz lohnend verbessert, ändert diese Fassung
nicht: `docs/vectors.json` führt byte-exakte Vektoren, und Cache-Keys hängen an
genau diesen Bytes. Sie begründet eine **nächste** Fassung mit einer neuen
Versionsnummer, nie eine stille Änderung dieser.

Der Hebel, den v0.2 hier vermutete (`B_min > 1`), ist außerdem nicht der, der
gewirkt hat. Gewirkt haben zwei andere: gar nicht erst hinzuschauen (§9.6) und
die Antwort hinschreiben statt sie zu rechnen (§9.2.4).

### 9.6 Eine Entscheidung am Dateikopf (normativ)

Das Programm aus §9.2 muss die Eingabe lesen, um zu erfahren, ob ein Literal
darin steckt. Wo keines steckt — und das ist alles, was ein Kompressor liefert
—, ist dieses Lesen Arbeit ohne Gegenwert, und zwar die teuerste, die der
Encoder hat.

> **Regel.** Vor dem Kodieren wird **einmal** entschieden, am Anfang der
> Eingabe:
>
> 1. Beginnt die Eingabe mit einer der Magic Numbers aus der Tabelle unten,
>    wird der ganze Strom als Base64URL geschrieben, ohne zu suchen.
> 2. Sonst, wenn die Eingabe mindestens **4096 Bytes** lang ist: die Shannon-Entropie
>    ihrer ersten 4096 Bytes wird in **Tausendstel Bit je Byte** als
>    Ganzzahl bestimmt. Ist sie **größer als 7400**, wird der ganze Strom als
>    Base64URL geschrieben, ohne zu suchen.
> 3. Sonst gilt §9.2 für die ganze Eingabe.

| Magic | Format |
|---|---|
| `1F 8B` | gzip |
| `28 B5 2F FD` | zstd |
| `FD 37 7A 58 5A` | xz |
| `42 5A 68` | bzip2 |
| `50 4B 03 04` | zip |
| `FF D8 FF` | JPEG |
| `89 50 4E 47` | PNG |
| `4F 67 67 53` | Ogg |
| `1A 45 DF A3` | Matroska / WebM |

**Ganzzahlig, nicht in Gleitkomma.** Diese Zahl entscheidet, welche Bytes der
Encoder schreibt; zwei Implementierungen müssen sich über sie exakt einig sein,
und Gleitkomma ist die Stelle, an der zwei Implementierungen aufhören, sich
einig zu sein. Der Logarithmus ist ganzzahlig zu bestimmen —
`conformance/reference.py` und die Rust-Implementierung tun es auf demselben
Weg, und §16.3 prüft, dass sie dasselbe herausbekommen.

**Warum die Stichprobe vollständig sein muss.** Über `n` Bytes kann die
Plug-in-Entropie von 256 Symbolen `log2(n)` nicht überschreiten; unter 4096
Bytes kann die Zahl den Schwellwert also gar nicht erreichen, wie zufällig die
Eingabe auch sei. Die Frage zu stellen wäre Arithmetik mit feststehendem
Ergebnis. Eine kürzere Eingabe bekommt deshalb das Programm — den gründlichen
Zweig, nicht einen Rückfall, und auf so wenig Bytes ist er billig.

**Die Ausgabe bleibt eine Funktion der Eingabe.** Magic Numbers sind ein
Präfixtest, die Stichprobe ist ein fester Präfix, die Schwelle ist eine Zahl,
der Logarithmus ist ganzzahlig. Es gibt nichts zu raten und nichts, was von der
Aufrufreihenfolge abhinge; §9.0 gilt unverändert.

**Eine falsche Entscheidung kostet Größe, nie Korrektheit.** Ein
übersprungener Strom ist exakt Base64URL, also greift §9.4 in jedem Fall. Das
ist die Eigenschaft, die es erlaubt, hier überhaupt zu raten — dieselbe
Begründung, die base91z in seinem §11.5 für dieselbe Entscheidung gibt.

**Einmal und nicht je Fenster.** base91z entscheidet je Fenster, und v0.2 hat
das übernommen, ohne die Begründung zu prüfen. Sie überträgt sich nicht:
base91z entscheidet, ob **komprimiert** wird, und eine falsche Entscheidung
kostet dort Größe auf jedem Byte. Hier wird entschieden, ob **hingeschaut**
wird, und eine falsche Entscheidung kostet nur Zeit. Gemessen über 101 Proben
— den ganzen Korpus samt Silesia und den kurzen Werten — kostet die eine
Entscheidung am Kopf gegenüber „immer suchen" **0,00 % Größe**, und keine
einzige Datei gibt mehr als einen halben Punkt auf.

**Zum Schwellwert.** 7400 Tausendstel Bit ist abgelesen, nicht hergeleitet. Ein
Literal braucht elf aufeinanderfolgende profil-legale Bytes, und Profil U lässt
66 von 256 zu; bei 7,4 Bit je Byte ist ein solcher Lauf verschwindend selten.
Wo genau die Schwelle liegen soll, hängt daran, was der Scan kostet, und das
ist eine Maschineneigenschaft — über den Korpus liegt sie so, dass die
Entscheidung mit „immer suchen" auf 0,00 % übereinstimmt.

## 10. Dekoder

### 10.1 Ablauf

```
pos := 0 ; alphabet_seen := none ; padding_seen := false
while pos < len:
    if stream[pos] == '~':
        prüfe: pos + 2 <= len                            sonst E_TRAILING_TILDE
        prüfe: stream[pos+1] ist Alphabetzeichen         sonst E_CHARSET        # (1)
        note_alphabet(stream[pos+1])                                            # (2)
        L1 := value(stream[pos+1])
        if L1 == 0:                                      -> E_RESERVED_LEN
        if L1 == 63:
            prüfe: pos + 4 <= len                        sonst E_TRUNCATED
            prüfe: stream[pos+2..pos+4] Alphabetzeichen  sonst E_CHARSET        # (1)
            note_alphabet(stream[pos+2]) ; note_alphabet(stream[pos+3])         # (2)
            L := 63 + (value(stream[pos+2])<<6 | value(stream[pos+3])) ; pos += 4
        else:
            L := L1 ; pos += 2
        prüfe: pos + L <= len                            sonst E_TRUNCATED
        prüfe: alle L Bytes profil-legal                 sonst E_PROFILE
        emit stream[pos .. pos+L]      # KEINE Alphabet-/Padding-Prüfung, §5.4/§5.3
        pos += L
    else:
        scanne bis zum nächsten '~' oder Stromende -> Segment der Länge m
        k := (Segment endet am Stromende) ? Anzahl '=' am Ende (max 2) : 0       # (3)
        n := m - k
        prüfe: k == 0 ∨ (k == 1 ∧ n mod 4 == 3) ∨ (k == 2 ∧ n mod 4 == 2)
                                                         sonst E_PADDING
        if k > 0: padding_seen := true
        prüfe: n mod 4 != 1                              sonst E_ALIGN
        prüfe: alle n Zeichen Alphabetzeichen            sonst E_CHARSET
        note_alphabet für jedes Zeichen mit Wert 62/63                           # (2)
        prüfe: Restbits des letzten Quantums == 0        sonst E_NONZERO_TAIL
        emit base64_decode(n chars) ; pos += m

note_alphabet(c):
    if c in {'+','/'}:  if alphabet_seen == url     -> E_MIXED_ALPHABET
                        else alphabet_seen := classic
    if c in {'-','_'}:  if alphabet_seen == classic -> E_MIXED_ALPHABET
                        else alphabet_seen := url
```

**(1)** Ohne diese Prüfung wird `value()` auf wertlosen Zeichen aufgerufen (`~~abc`,
`~=ab`) — undefiniert oder Lookup außerhalb der Tabelle. **(2)** implementiert
Regel A. **(3)** implementiert Regel P; die Bedingung „endet am Stromende" hält `=`
aus der Vektor-Schleife heraus und verhindert, dass ein `=` als letztes Literal-Byte
in Profil T fälschlich als Padding gelesen wird (TV10).

Die vierte Implementierungsfalle, die v0.2 hier führte, betraf die
Marker-Prüfung des Framed Mode und ist mit ihm entfallen (§8).

### 10.2 Einstiegspunkt

```
decode(stream, profile)
decode_url_strict(stream, profile)      # weist '+' und '/' mit E_NON_URL_ALPHABET ab
```

Zwei Funktionen, kein Modus dazwischen. v0.2 hatte drei plus eine
Vorabentscheidung (§5.6); die ist mit dem Framed Mode entfallen und mit ihr die
einzige Stelle, an der ein Angreifer durch die Wahl der ersten zwei Zeichen
bestimmte, welcher Parser läuft.

### 10.3 Framed Mode — **zurückgezogen**

Siehe §8.

### 10.4 Fehlerfälle

| Code | Bedingung |
|------|-----------|
| `E_TRAILING_TILDE` | Strom endet mit `~` oder unvollständigem Header |
| `E_RESERVED_LEN` | `L1 == 0` |
| `E_TRUNCATED` | Payload reicht über das Stromende hinaus |
| `E_PROFILE` | Literal-Byte außerhalb des Profil-Alphabets |
| `E_ALIGN` | Base64-Segmentlänge `mod 4 == 1` |
| `E_NONZERO_TAIL` | Restbits im letzten Quantum ≠ 0 |
| `E_CHARSET` | kein Alphabetzeichen an Alphabetposition (inkl. `~`, Header, `=` außerhalb des Stromendes) |
| `E_PADDING` | Regel P verletzt |
| `E_MIXED_ALPHABET` | Regel A verletzt |
| `E_NON_URL_ALPHABET` | nur `decode_url_strict` |

Zehn statt zwölf: `E_FRAME_RULE` und `E_FRAME_SYNC` sind mit §8 entfallen.

**Allokationsgrenzen.** Die Literallänge ist hart auf 4158 Bytes begrenzt. Daraus
folgt: die Spezifikation braucht **kein protokollseitiges Limit für einzelne
Segmente**, und es gibt keine varint-Längen mit der zugehörigen Klasse von
Einzelallokations-Bugs.

Daraus folgt **nicht**, dass gar kein Limit nötig wäre. Die Zahl der Segmente ist
unbegrenzt; ein Strom kann beliebig groß werden und beliebig große kumulative
Ausgabe erzeugen. Implementierungen SOLLTEN Gesamtgrößen- und Laufzeitlimits
anbieten.

## 11. Kanonizität und Signaturen

**Der Encoder ist eine Funktion** (§9.0): (Eingabe, Profil) bestimmt den Strom
eindeutig, es gibt kein Preset mehr, das dazwischenträte, und ein Encoder
schreibt nur das URL-Alphabet und nie Padding (§5.1, §5.3).

Kanonisch ist das *Format* damit trotzdem nicht, aus zwei verbleibenden Gründen.
Erstens ist das **Profil eine Wahl**: derselbe Input ergibt unter U und T
verschiedene Ströme. Zweitens akzeptiert der **Dekoder Formen, die kein Encoder
schreibt** — das Classic-Alphabet (§5.2) und Padding (§5.3). Ein Dritter kann
denselben Strom also umschreiben, ohne die dekodierten Bytes zu ändern. Regel A
und Regel P halten diese Freiheit bei je Faktor 2.

> **Regel:** Signiere, hashe und vergleiche niemals die Ausgabe von `encode`.
> Signiere die **dekodierten Bytes**. `decode(encode(x)) == x` gilt immer.

**Was v0.4 dagegen zusichert und was ein Cache-Key braucht:** dass zwei
konforme Encoder für dieselbe Eingabe und dasselbe Profil dieselben Bytes
schreiben. Das reicht für Cache-Keys, Dedup-Keys und Content-Adressen — dort
erzeugt und vergleicht dieselbe Seite —, aber nicht für Signaturen, wo ein
Angreifer den Strom liefert.

**Base64 bleibt die ehrlichere Wahl,** wenn ein *fremdes* Protokoll die kodierte Form
signiert: es hat keine Parameter.

### 11.1 Die Ordnung

#### Warum eine Byte-Klassifikation nicht reicht

Eine Ordnung über einem Bitvektor `isLiteral ∈ {0,1}^n` wäre nicht total, denn
`isLiteral` bestimmt die Ausgabe nicht eindeutig: Zwei angrenzende LiteralSegmente
sind nach §4 erlaubt und vom Dekoder unterscheidbar. Ein Literal-Lauf von `m` Bytes
kann deshalb als **ein** Segment oder als **mehrere** kodiert werden — bei
identischem `isLiteral`. Mit `h` aus §6.1 gilt für einen Lauf von `m = m₁ + m₂`:

```
ein Segment  : m + h(m)
zwei Segmente: m + h(m₁) + h(m₂)
```

Für `63 ≤ m ≤ 124` mit `m₁, m₂ ≤ 62` ist `h(m) = 4 = h(m₁) + h(m₂)` — **exakter
Gleichstand**:

| `m` | ein Literal | zwei Literale | `isLiteral` | Kosten |
|-----|-------------|---------------|-------------|--------|
| 100 | 104 chars | 50+50 → 104 chars | identisch | **gleich** |
| 124 | 128 chars | 62+62 → 128 chars | identisch | **gleich** |
| 200 | 204 chars | 100+100 → 208 chars | identisch | verschieden |

`isLiteral` klassifiziert Bytes, legt aber keine Segmentgrenzen fest.

#### Drei-Symbol-Vektor

Eine Segmentierung `S` wird durch einen Vektor über drei Symbolen beschrieben:

```
c[0 .. n-1] ∈ {B, L, S}^n

B  Byte i liegt in einem Base64-Segment
S  Byte i ist das erste Byte eines Literal-Segments   (Start)
L  Byte i setzt ein Literal-Segment fort              (Lauf)
```

`c` legt die Segmentgrenzen vollständig fest. Zusammen mit

* maximalen Base64-Läufen (§4) — ein Aufteilen ist nicht darstellbar,
* kürzester Header-Form (§6.1),
* Alphabet URL und ausgeschlossenem Padding (§5.1, §5.3)

ist `output(S)` damit eine **Funktion** von `c`. Eine Sonderregel für das Teilen von
Literal-Läufen > 4158 ist nicht nötig: solche Teilungen sind in `c` als zusätzliche
`S` sichtbar und werden von der Ordnung unten entschieden. Das ist auch sachlich
nötig — ein Teilen von links ist nicht immer längenoptimal (für `m = 4300` kostet
`4158+142` acht Header-Zeichen, `4238+62` nur sechs).

#### Ordnung

`encode(x, profil)` ist das Minimum von

```
Key(S) = ( |output(S)| ,  c(S) )
```

über alle im Profil zulässigen `S`. Erste Komponente numerisch, zweite
**lexikographisch von Index 0 aufsteigend** mit

```
B  <  L  <  S
```

Das kodiert beide Präferenzen an der frühesten abweichenden Position:

* `B < L, S` — bei gleicher Gesamtlänge gewinnt **Base64** gegenüber einem Literal.
* `L < S` — innerhalb eines Literal-Laufs gewinnt **Fortsetzen** gegenüber einem
  neuen Start, also das **maximale** Literal-Segment.

`{B,L,S}^n` ist bei festem `n` total geordnet, das Minimum also eindeutig.

**Die Ordnung ist innerhalb eines festgelegten Profils kanonisch**, nicht darüber
hinaus. Zwei Aufrufe mit verschiedenem Profil liefern verschiedene Keys — korrekt,
da das Profil Teil des Container-Vertrags ist (§7.2).

#### Berechnung

Rückwärtslauf des DP aus §9.2 liefert `Restkosten[j]`; ein Vorwärtslauf
rekonstruiert, indem er an jeder Position das **kleinste Symbol** wählt, das noch
eine längenoptimale Fortsetzung zulässt:

* `B`, wo ein Base64-Segment optimal beginnen kann;
* sonst `L`, solange das Literal optimal weiterlaufen kann;
* sonst `S`.

Das ist die Ordnung selbst, angewandt Position für Position, und deshalb ihr
Minimum.

**Nicht** „das längste zulässige Literal". Ein Literal früh zu beenden richtet
den Base64-Lauf dahinter so aus, dass später noch ein Literal längenoptimal
wird, und `B < L` entscheidet dann für das kürzere; TV12 ist das kleinste
Beispiel. v0.1 stand hier anders und war an dieser Stelle mit sich selbst im
Widerspruch (`docs/history/errata-v0.1.de.md`, E1).

**Zur Laufzeit.** Der Rückwärtslauf ist O(n) nach §9.2. Für den Vorwärtslauf ist
keine O(n)-Schranke bewiesen: die Deques liefern das Minimum, die Rekonstruktion
braucht das Argument des Minimums unter einer Tie-Break-Regel, und das ist eine
andere Anfrage. Eine Aufzählung der zulässigen Enden je Literal kostet
O(Fenster) je Literal.

**Verifikation.** Gegen erschöpfende Aufzählung **aller** gültigen
Segmentierungen bis `n ≤ 12`, über beide Profile und über Alphabete, die
profil-legale und -widrige Bytes mischen: keine Abweichung zwischen DP und Brute
Force, bei mehr als fünfzig Eingaben mit echtem Längen-Gleichstand.

Die Fassung in v0.1 reichte bis `n ≤ 9` und fand deshalb nichts: die kleinste
Eingabe, auf der die alte *Berechnung* von der Ordnung abweicht, ist zehn Bytes
lang (TV12). Eine Suchraumgrenze ist eine Behauptung darüber, wo die Antwort
liegt, und gehört neben das Ergebnis.

#### Kein `L_min` — und wie die Fensterung hineinpasst

Die Ordnung kennt keinen Schwellwert (§9.1). Ein Literal wird genommen, wo es
kürzer ist, bei günstiger Ausrichtung bis hinab zu sieben Bytes. Eine spätere
Messung DARF daran nichts ändern (§9.5), sonst bewegten Messergebnisse
rückwirkend bestehende Cache-Keys.

Die Fensterung aus §9.2.1 kennt sie dagegen sehr wohl, und das ist normativ:

> `encode(x, profil)` ist die Ausgabe der Segmentierung, die entsteht, wenn man
> `x` an den absoluten Offsets `k · 65536` teilt, auf **jedes** Fenster das
> Minimum von `Key` anwendet, die Ergebnisse aneinanderhängt und dabei
> angrenzende Base64-Segmente zu einem verschmilzt (§4).

Für `len(x) ≤ 65536` — jeden Wert, den §0.1 nennt, und jeden Testvektor — ist
das genau das Minimum über die ganze Eingabe. Darüber kann es um bis zu vier
Zeichen je Fenstergrenze länger sein, weil ein Literal keine Grenze überspannt;
§9.4 gilt trotzdem, weil auch der reine Base64-Kandidat über die Grenze hinweg
derselbe ist.

Die Fenstergröße gehört damit zur Definition und nicht zur Implementierung. Das
ist der Preis dafür, dass eine Implementierung mit konstantem Speicher
auskommt, und er ist bewusst hier bezahlt und nicht in §9.4: eine
Implementierung, die das Programm über eine Gigabyte-Eingabe am Stück laufen
lässt, ist **nicht** konform, auch wenn ihre Ausgabe minimal kürzer wäre.

**Verwendung:** Cache-Keys, Dedup-Keys, Content-Addressing, Testvektoren.
**Nicht** für Signaturen — dort gilt §11.


## 12. Dichte

Die beiden mittleren Zeilen sind exakt, die beiden unteren gemessen auf
erzeugten Eingaben der angegebenen Form (`cargo run --release --example
density`, 1 MiB je Zeile). Erzeugte Eingaben sind kein Korpus: die Zahl hängt
daran, wie gemischt wird, und deshalb verlangt §16.5 dafür binary2textbench.

| Eingabe | Base64 | **Base65t/U** | Base65t/T | §9.6 | Z85 | basE91 |
|---------|--------|---------------|-----------|------|-----|--------|
| Rein binär | 1,333 | **1,333** *(exakt)* | 1,333 | Base64 | 1,250 | 1,231 |
| Rein profil-legaler Text | 1,333 | **≤ 1,00096** *(exakt)* | ≤ 1,00096 | Exact | 1,250 | 1,231 |
| 70 % Text / 30 % binär | 1,333 | *1,113* | *1,112* | Exact | 1,250 | 1,231 |
| 30 % Text / 70 % binär | 1,333 | *1,333* | *1,333* | Base64 | 1,250 | 1,231 |

Die letzte Zeile ist ein Beispiel dafür, wie §9.6 wirkt: bei 70 % Binäranteil
liegt die Entropie der Stichprobe über der Schwelle, der Encoder schaut nicht
hin, und die Ausgabe ist **byteweise** Base64URL. Immer-Suchen käme hier auf
1,299 — 2,5 % besser, für den vollen Scan. Über den echten Korpus ist derselbe
Unterschied 0,00 % (§9.6).

Zur zweiten Zeile: `4162/4158 = 1,00096` gilt für einen maximalen Literalblock und
ist eine **exakte Schranke**, kein Grenzwert. Da Literale bei 4158 Bytes gedeckelt
sind, nähert sich die Dichte langer Eingaben dieser Konstanten an, nicht der 1.

**URL-Query — gilt ausschließlich für Profil U:**

| Container | Base64url | **Base65t/U** | Base65t/T | Base85N | Base91z |
|-----------|-----------|---------------|-----------|---------|---------|
| URL-Query | 1,333 | **≤ 1,333, bei Text bis 1,001** | prozent-encoding-pflichtig | *(1,463 über Korpus)* | nicht geeignet |

Der URL-Vorteil ist ein Vorteil **von Profil U**, nicht des Formats an sich.

**Über den Korpus** (69 Proben aus binary2textbench, `--example gain`):
Profil U kommt auf **98,57 %** der Base64-Größe, Profil T auf **93,60 %**. Die
Verteilung ist keine Kurve, sondern zwei Populationen: 55 % der Proben liegen
unter 95 %, 19 % sind von Base64 nicht zu unterscheiden, und dazwischen liegt
fast nichts. Das ist die ehrlichste Aussage über den Nutzen dieses Formats —
es hilft dort, wo die Eingabe **schon Text ist**, und sonst nirgends.

## 13. Performance

Base64 hat null datenabhängige Branches, Base65t einen pro Segmentwechsel. Bei fein
durchmischten Daten ist Base65t deshalb langsamer. Alle Zahlen unten sind gegen
die Base64-Implementierung des Benches gemessen, die im selben Prozess lebt,
vom selben Compiler mit denselben Schaltern gebaut wurde und dieselbe
Schleifenform hat — der Vergleich misst das Format und kein Handicap.

### 13.1 Die Vektor-Schleife

Die Schleife ist **zustandsbehaftet**. Ohne diese Unterscheidung liest sie eine
Daten-Tilde als Segmentgrenze.

```
state := BASE64
while pos < len:
    if state == BASE64:
        # Nur hier wird gescannt.
        mask := _mm256_movemask_epi8(_mm256_cmpeq_epi8(load32(pos), splat('~')))
        if mask == 0:
            base64_shuffle_32(...) ; pos += 32
        else:
            X := tzcnt(mask)
            base64_decode_tail(pos, X)        # Quantum am Segmentende schließen
            (headerlen, L) := read_header(pos + X)
            pos += X + headerlen ; remaining := L ; state := LITERAL
    else:  # LITERAL
        # KEIN Scan, KEIN Vergleich gegen '~'. Die Länge ist die einzige Grenze.
        memcpy(out, stream + pos, remaining)
        pos += remaining ; state := BASE64
```

**Quanten-Hinweis (§4).** Der 32-Byte-Shuffle ist relativ zum **Segmentanfang**
ausgerichtet; am Segmentende schließt `base64_decode_tail` das angebrochene Quantum.
Wer den Shuffle über die Grenze laufen lässt, produziert stillschweigend falsche
Bytes — kein Fehlercode fängt das ab.

### 13.1.1 Was Vektorisierung bringt, und warum Regel A sie erlaubt

Die Referenzimplementierung kann das Base64-Schreiben **und das
Base64-Lesen** hinter dem Feature `simd` an einen vektorisierten Kern abgeben.
Die Ausgabe ändert sich dabei nicht um ein Byte — Base64 ist Base64 — also ist
es ein Geschwindigkeitsschalter, kein Formatthema.

**Beim Dekoder ist das nicht selbstverständlich.** Eine Base64-Bibliothek legt
sich je Aufruf auf **ein** Alphabet fest und meldet **einen** Fehler; §5.2
verlangt, dass beide Varianten gelesen werden, §5.4 verlangt zu wissen, welche
es war, und §10.4 verlangt zehn unterscheidbare Bedingungen. Was das rettet,
ist eine Beobachtung: Regel A braucht nur die Frage *„steht in diesem Lauf ein
`+`, `/`, `-` oder `_`"*, und das ist eine **Suche und keine Dekodierung**. Als
eigener Durchgang gefragt kostet sie ein Siebtel dessen, was das Dekodieren
kostet — und ihre Antwort wählt anschließend das Alphabet für den
Bibliotheksaufruf. Bleibt der Fehlercode: schlägt der vektorisierte Aufruf
fehl, läuft die skalare Schleife noch einmal über denselben Lauf und benennt
die Bedingung. Das ist per Definition der langsame Pfad — er läuft einmal, auf
einem Strom, der ohnehin abgelehnt wird.

Der Tail-Bit-Test aus §5 (`E_NONZERO_TAIL`) ist kein Hindernis: eine
ordentliche Bibliothek prüft ihn selbst.

**Auf den Lauflängen, die dieses Format wirklich erzeugt** (nicht auf einem
Acht-Megabyte-Aufruf, wie SIMD-Bibliotheken üblicherweise beworben werden):

| Lauflänge | 16 B | 40 B | 63 B | 128 B | 366 B | 16 KiB |
|---|--:|--:|--:|--:|--:|--:|
| vektorisiert / skalar | 1,1× | 1,6× | 2,0× | 2,5× | 3,5× | 3,7× |

Ein Segment ist im Korpus zwischen 63 (Tar) und 1852 Bytes (Wasm) lang, also
liegt der Gewinn bei 2 bis 3,5 und nicht bei den 10, die eine Spitzenzahl
verspricht.

### 13.2 Das Durchsatz-Kriterium

> **Durchsatz ist ein Ziel, Größe ist eine Zusicherung.** Eine Änderung DARF
> die Zusicherung aus §9.4 und die Bytegleichheit aus §11.1 nicht antasten.
> Innerhalb dieser Schranke SOLL sie den Durchsatz verbessern; eine Änderung,
> die Durchsatz gegen Dichte tauscht, braucht eine neue Versionsnummer.

Woran das gemessen wird:

* **Auf hochentropen Daten schreibt der Encoder dieselben Bytes wie
  Base64URL** und schaut sie nach §9.6 gar nicht erst an. Dort ist Parität
  keine Zusage, sondern Identität.
* **Für alles andere berichtet die Bench**, mit den Zahlen unten und einem
  Korpus, der veröffentlicht ist.
* **Skalar gegen vektorisiert ist kein Ergebnis.** Ein Vergleich zählt, wenn
  beide Seiten denselben Grad an Handarbeit gesehen haben.
* **Die eigene Schleife ist zuerst verdächtig, nicht das Format.** Der Abstand
  beim Dekodieren war einmal 158 % und lag zu keinem Teil am Format: er lag an
  zwei Schleifen der Referenzimplementierung, die byteweise arbeiteten, wo das
  Format es nicht verlangt.

Ein Encoder oder Dekoder ist nicht deshalb unkonform, weil er langsamer ist als
Base64. Er ist es, wenn er die falschen Bytes schreibt.

### 13.3 Was das exakte Programm kostet — der offene Punkt dieser Fassung

Große Dateien, Profil U, einthreadig, MiB/s und Base65t als Anteil an der
Base64-Zeit (`--example large`):

| Datei | §9.6 | Größe | Base64 enc | Base65t enc | enc | Base64 dec | Base65t dec | dec |
|---|---|--:|--:|--:|--:|--:|--:|--:|
| `dickens` | Exact | 98,8 % | 512 | 45 | **1137 %** | 801 | 485 | **165 %** |
| `mozilla` | Exact | 98,3 % | 393 | 62 | **636 %** | 539 | 529 | **102 %** |
| `xml` | Exact | 98,0 % | 507 | 55 | **922 %** | 847 | 571 | **148 %** |
| `x-ray` | Exact | 100,0 % | 534 | 69 | **778 %** | 825 | 794 | **104 %** |
| `countries.json` | Exact | 99,3 % | 601 | 63 | **948 %** | 902 | 674 | **134 %** |

**Das ist teuer, und es ist der Preis für genau einen Satz dieser Fassung:**
dass es einen Encoder gibt und nicht fünf. v0.2 hatte hier eine lineare Regel
für den Default, die 105 bis 125 % kostete und 0,22 % Größe aufgab; sie ist
weg, weil eine nicht längenoptimale Regel §11.1 nicht erfüllen kann und §11.1
die Bytegleichheit trägt, an der Cache-Keys hängen (§0.1).

Was den Schaden begrenzt, sind die zwei Zweige, in denen das Programm gar nicht
läuft:

* **§9.6** fängt alles ab, was komprimiert oder hochentropisch ist. Auf
  `random.bin`, jedem Archiv, jedem Bild ist der Encoder byteweise Base64URL
  und kostet dessen Zeit.
* **§9.2.4** fängt ab, was durchgehend profil-legal ist — die kurzen Werte, für
  die das Format gemacht ist (§13.4).

Übrig bleibt gemischter Text im Megabyte-Bereich: `dickens` in Profil U ist der
schlechteste Fall, den der Korpus kennt, und er ist auch der, in dem das Format
mit 98,8 % am wenigsten holt. **Wer solche Daten hat, gewinnt hier nichts und
sollte Base64 nehmen** — oder Profil T, wo derselbe Text auf 79,9 % kommt.

**Als offener Punkt festgehalten:** 45 MiB/s ist nicht die Schranke des
Algorithmus, sondern der Stand einer Implementierung. Der Rückwärtslauf ist
O(n) mit zwei Schiebefenster-Minima, und was er kostet, sind die
unvorhersagbaren Sprünge in den beiden Deques. Eine verzweigungsfreie Fassung
ist denkbar und nicht versucht. Diese Fassung sagt die Zahl, statt sie zu
verschweigen; sie ist kein Konformitätskriterium (§13.2).

### 13.4 Auf kurzen Werten ist Base65t schneller als Base64

Die Tabelle oben misst Dateien von mehreren Megabyte. Dafür ist das Format
nicht gemacht: §0.1 nennt URL-Query, Cookie-Wert, HTTP-Header und Cache-Key,
und keiner davon ist acht Megabyte groß.

Dieselben 55 kurzen Proben, die `binary2textbench` als `short/` führt, Profil
U. Die Größenspalte steht gegen `ceil(4n/3)` — das ist, was §9.4 zusichert und
was eine URL wirklich trägt; die Zeitspalten stehen gegen die Base64 des
Benches, die paddet, was auf so kurzen Werten für die *Größe* ein Viertel
ausmachen kann und für die *Zeit* nichts:

| Probe | Bytes | §9.2.4 | Größe | Kodieren | Dekodieren |
|---|--:|---|--:|--:|--:|
| SHA-512-Digest, hex | 128 | ja | 77 % | **38 %** | **73 %** |
| JWT, drei Segmente | 155 | ja | 77 % | **45 %** | **69 %** |
| zwei ULIDs | 52 | ja | 77 % | **56 %** | **74 %** |
| SHA-256-Digest, hex | 64 | ja | 78 % | **53 %** | **92 %** |
| AES-256-Schlüssel, hex | 64 | ja | 78 % | **52 %** | **87 %** |
| Session-ID, 40 alnum | 40 | ja | 78 % | **58 %** | **77 %** |
| UUID v4 | 36 | ja | 79 % | **64 %** | **77 %** |
| ULID, Crockford | 26 | ja | 80 % | **71 %** | **79 %** |
| Kreditkartennummer | 16 | ja | 82 % | **77 %** | **86 %** |
| IPv4-Adresse | 11 | ja | 87 % | **92 %** | **90 %** |
| Vor- und Nachname | 12 | nein | 100 % | 598 % | 129 % |
| IPv6-Adresse | 28 | nein | 100 % | 693 % | 120 % |
| Logzeile | 93 | nein | 95 % | 816 % | 143 % |
| SQL-Statement | 118 | nein | 98 % | 830 % | 132 % |
| zufällige 64 Bytes | 64 | nein | 100 % | 722 % | 114 % |
| **alle 55 Proben, als Zeit** | | | | **355 %** | **103 %** |

**Die Spalte §9.2.4 erklärt die ganze Tabelle.** Wo jedes Byte profil-legal
ist, steht die Antwort ohne Programm fest, und der Encoder ist schneller als
Base64 — weil er weniger schreibt. Wo ein Leerzeichen, ein Doppelpunkt oder ein
`=` dazwischensteht, läuft das Programm, und die Zeile kostet das Sechs- bis
Achtfache. Dass das genau die Zeilen sind, in denen die Größe bei 95 bis 100 %
liegt, ist kein Zufall: es ist dieselbe Eigenschaft, von zwei Seiten gesehen.

Die Summenzeile ist nach Zeit gewichtet und wird deshalb von der unteren Hälfte
bestimmt: 355 % über alle 55 Proben. Sie steht hier, weil sie die ehrliche Zahl
ist, aber sie beschreibt keine der beiden Populationen. Wer kurze Werte kodiert,
die schon Text sind, liest die obere Hälfte; wer gemischten Text kodiert, liest
die untere und §13.3 dazu.

**Der Durchsatzvorteil *ist* der Dichtevorteil.** Base64 liest ein Byte,
schlägt vier Zeichen nach und schreibt vier — je drei Bytes. Ein Literal liest
ein Byte, prüft es gegen die Profilmenge und schreibt **ein** Zeichen; das
Schreiben ist ein `memcpy`. Wer weniger schreibt, schreibt schneller.

> **Als Faustregel, aus der Bilanz und nicht aus einer Messung:** wo Base65t
> kürzer ist, ist es ungefähr so viel schneller — und wo es nicht kürzer ist,
> ist es um das Suchen langsamer.

**Und dieselbe Zeile in Profil T.** Eine Logzeile, ein SQL-Statement und eine
IPv6-Adresse sind in Profil T durchgehend legal, fallen also unter §9.2.4 und
wechseln von der unteren Hälfte der Tabelle in die obere. Wer diese Werte hat,
hat auch den Container, der T trägt — ein Logfeld, ein JSON-String —, und §7
sagt, was er dafür aufgibt.

### 13.5 Lesbarkeit, und was das Profil daran ändert

Ein Literal steht im Ausgabestrom, wie es in der Eingabe stand. Wie viel davon
lesbar bleibt, entscheidet nicht der Encoder, sondern das Profil — und zwar um
Größenordnungen mehr, als je eine Encoder-Regel es getan hat:

| Datei | Größe U | Größe T | Klartext U | Klartext T |
|---|--:|--:|--:|--:|
| `xml` (Silesia) | 98,0 % | **79,8 %** | 21 % | **92 %** |
| `dickens` (Silesia) | 98,8 % | **79,9 %** | 17 % | **91 %** |
| `lodash.js` | 97,9 % | **81,9 %** | 23 % | **88 %** |
| `bootstrap.css` | 92,5 % | **82,6 %** | 54 % | **88 %** |
| `countries.json` | 99,3 % | **94,0 %** | 9 % | **47 %** |

Der Grund ist das Leerzeichen. In Profil U ist es nicht legal, also zerfällt
englische Prosa in fünf Byte lange Läufe, von denen keiner ein Literal wert
ist; in Profil T ist es legal, und derselbe Text wird ein einziges Literal.
v0.2 hatte für diese Frage ein eigenes Preset (`legible`), das fünf Punkte
brachte und allen anderen Presets 60 bis 190 % der Zeit kostete. Das Profil
bringt siebzig Punkte und kostet nichts.

## 14. Sicherheit

* **Der Dekoder parst angreiferkontrollierte Längen. Base64 tut das nicht.** Ein
  Nachteil gegenüber Base64. Zurückgezahlt durch die harte 4158-Byte-Grenze, die
  Prüfliste in §10.4, Fuzzing und speichersichere Referenzimplementierungen. Eine
  speichersichere Sprache ist die *Bezahlung* dieser Angriffsfläche, kein Argument
  gegen Base64.
* **Literale lecken Struktur** — Klartextanteil und alle Lauflängen sind sichtbar.
  Dafür ist `encode_base64url` da (§9.3); seine Ausgabe ist gewöhnliches
  Base64URL.
* **Zwei Auto-Erkennungen sind zwei Parser-Differential-Flächen:** Alphabet
  (§5.2) und Padding (§5.3). v0.2 hatte drei; die dritte war das Framing, und
  sie ist mit §8 entfallen. Liest eine Komponente permissiv und eine andere
  strikt, entstehen zwei Wahrheiten über denselben Strom. Gegenmaßnahmen: Regel
  A, Regel P, `alphabet_seen` / `padding_seen` und `decode_url_strict` (§5.5).
  Differential-Fuzzing ist Pflicht, nicht Kür.
* **Kein Padding-Orakel** — Padding wird nur validiert, nie erzeugt.
* **Malleability** ausgeschlossen auf Segmentebene, reduziert auf Alphabet- und
  Padding-Ebene, **nicht** auf Profilebene (§11).
* Dekodierte Ausgabe ist **untrusted binary**, nicht Text.

## 15. Testvektoren

Dreizehn Vektoren, jeder als Test in `rust/tests/vectors.rs`. Der maschinell
prüfbare Satz — 137 Einträge über beide Einstiegspunkte und beide Profile, je
als Eingabe und erwarteter Strom in Hex — steht in `docs/vectors.json` und wird
von `rust/examples/vectors.rs` erzeugt.

Zurückgezogen gegenüber v0.2: TV5b, TV9a, TV9b, TV11 und TV15 beschrieben den
Framed Mode (§8), TV14 das gestrichene Preset `legible`. Die verbleibenden sind
neu durchnummeriert; wer eine alte Nummer sucht, findet sie in
`docs/history/spec-v0.2.de.md`.

### TV1–TV4 — Grundfälle (Profil U)

| # | Eingabe | Strom | Länge | Base64 wäre |
|---|---------|-------|-------|-------------|
| TV1 | `alice.jones` | `~Lalice.jones` | 13 | 15 |
| TV2 | `DE AD BE EF` + `session-eu-central` | `3q2-73Nl~Qssion-eu-central` | 26 | 30 |
| TV3 | `sub~alice~jones` | `~Psub~alice~jones` | 17 | 20 |
| TV4 | 100 × `a` | `~_Al` + 100 × `a` | 104 | 134 |

**Zu TV2.** Drei Segmentierungen sind hier 26 Zeichen lang: `ceil(4k/3) + (22−k)
+ 2` ist für `k = 4, 5, 6` gleich 26. v0.1 druckte hier
`3q2-7w~Ssession-eu-central` — den `k = 4`-Strom, den ein scannender Encoder
schreibt. Er ist gleich lang und dekodiert zu denselben Bytes; er ist nur nicht
das Minimum von `Key`, und seit §9.0 den Tie-Break vorschreibt, ist das der
Unterschied zwischen konform und nicht.

**Zu TV4.** `L1 = 63` (`_`), dann zwölf Bit `V = 37 = 000000 100101`, also `A`
und `l`. Im Classic-Alphabet lautet derselbe Header `~/Al`, und ein Dekoder
nimmt ihn (§5.2).

### TV5 — `~A` in einem Literal

`hello~Alice` war die Eingabe, an der v0.1 den F1/F2-Konflikt aufhängte: ein
Frame-Body durfte `~A` nicht tragen, also musste der Encoder das Literal
aufbrechen und Base64 schreiben, das länger war als das Literal, das es
ersetzte.

v0.4 hat keine Frames, also gibt es keinen Konflikt und keine Regel: das
Literal gewinnt, `~A` ist Nutzlast wie jedes andere Bytepaar.

```
encode("hello~Alice", U)  =  ~Lhello~Alice        13 chars
encode_base64url(...)     =  aGVsbG9-QWxpY2U      15 chars
```

### TV6 — Abwärtskompatibilität

| Strom | Bytes | `alphabet_seen` | `padding_seen` |
|-------|-------|-----------------|----------------|
| `PDw_Pz8-Pg` | `<<???>>` | url | false |
| `PDw/Pz8+Pg` | `<<???>>` | classic | false |
| `YWxpY2Uuam9uZXM` | `alice.jones` | none | false |
| `YWxpY2U=` | `alice` | none | true |

### TV7 — Alphabet-Konsistenz

`PDw_Pz8+Pg` und `PDw/Pz8-Pg` → `E_MIXED_ALPHABET`. `decode_url_strict` auf
`PDw/Pz8+Pg` → `E_NON_URL_ALPHABET`.

Die negative Hälfte, und die, an der ein Gesamtstrom-Scanner scheitert:

```
decode("~Ka+b/c-d_e~fg", T)  =  "a+b/c-d_e~~"   alphabet_seen = none
```

Die Zeichen einer Literal-Payload sind Daten.

### TV8 — Header-Zeichenvalidierung

`~~abc`, `~=ab`, `~_A~` → `E_CHARSET`. `~` → `E_TRAILING_TILDE`.
`~A` → `E_RESERVED_LEN`.

### TV9–TV10 — Padding

```
YWxpY2U=     -> "alice",  padding_seen
YWxpY2Uu     -> "alice.", kein Padding
YWxp==       -> E_PADDING
YWxpY2U==    -> E_PADDING
YWxpY2U=~Lfoo-> E_CHARSET      (= nicht am Stromende)
```

TV10, Profil T, und der Grund, warum Padding nicht vorab gestrippt werden darf:

```
~Da=b=   -> E_PADDING      (drei Payload-Bytes, das letzte '=' steht am Stromende)
~Ea=b=   -> "a=b=",  kein Padding
```

Beide Ströme enden auf `=`; nur die Literallänge entscheidet, ob der Scanner es
je ansieht.

### TV11 — Fehlerfälle

| Strom | Code |
|-------|------|
| `abcde` | `E_ALIGN` |
| `~A` | `E_RESERVED_LEN` |
| `~Labc` | `E_TRUNCATED` |
| `~Cab~` | `E_TRAILING_TILDE` |
| `YWxp==` | `E_PADDING` |
| `~Ca b` | `E_PROFILE` (Profil U) |
| `YWxpY2V` | `E_NONZERO_TAIL` |

`YWxpY2V` ist `alice` mit einem gesetzten Bit im letzten Quantum; kanonisches
Base64 schriebe `YWxpY2U`.

### TV12 — die Tie-Break-Regel (§9.0, §11.1)

Die kleinste Eingabe, auf der die Ordnung überhaupt etwas entscheidet: neun
profil-legale Bytes, dann eines, das es nicht ist.

```
Eingabe:              "aaaaaaaaa "        (10 Bytes)
encode(x, U):         ~HaaaaaaaYWEg       13 chars   c = SLLLLLLBBB
v0.1s "Berechnung":   ~JaaaaaaaaaIA       13 chars   c = SLLLLLLLLB
```

Beide sind dreizehn Zeichen lang und dekodieren zur Eingabe. An Index 7 steht
`B` gegen `L`, und `B < L` nimmt das kürzere Literal — es früh zu beenden
richtet den Base64-Lauf so aus, dass die restlichen drei Bytes zwei Zeichen
kosten statt vier. Der zweite Strom ist, was v0.1s *Berechnung* erzeugte
(E1 der Errata).

### TV13 — die Entscheidung am Kopf (§9.6)

Der Vektor ist die Entscheidung und kein Strom, weil dort eine zweite
Implementierung falsch abbiegt:

```
classify("\x1f\x8b\x08\x00\x00\x00\x00\x00")  =  Base64      (gzip, Magic)
classify("\x28\xb5\x2f\xfd\x00\x00\x00\x00")  =  Base64      (zstd, Magic)
classify("alice.jones")                       =  Exact       (< 4096 Bytes)
classify(4096 × 'a')                          =  Exact       (H = 0)
classify(4096 × 'a' + 100000 zufällige Bytes) =  Exact       (Stichprobe ist Präfix)
```

Die Schwelle ist 7400 Tausendstel Bit je Byte, die Stichprobe die ersten 4096
Bytes, der Logarithmus ganzzahlig. Zwei Encoder, die hier verschieden
entscheiden, schreiben für dieselbe Eingabe verschiedene Bytes.

## 16. Konformitätsnachweise

Eine Implementierung gilt als konform, wenn sie die vier folgenden Eigenschaften
belegt:

1. **`decode(encode(x)) == x`** für beide Profile, über einen Fuzzing-Korpus.
2. **`decode(base64(x)) == x`** und **`decode(base64url(x)) == x`** für alle
   kanonischen Eingaben, gepaddet und ungepaddet — per Differential-Fuzzing gegen die
   Standard-Base64-Bibliothek der jeweiligen Sprache. Erwartete Abweichungen
   (`E_NONZERO_TAIL`, §1.1) gehören als solche in den Korpus.
3. **`encode(x, profil)` byte-identisch über zwei unabhängige
   Implementierungen**, über den gesamten Vektorsatz. Ohne diesen Test ist §11.1
   eine Behauptung.
   **Erbracht, mit einer benannten Lücke.** Zwei Implementierungen liegen bei:
   `rust/` und `conformance/reference.py`, die zweite aus diesem Dokument
   geschrieben, ohne die Schiebefenster aus §9.2 und ohne eine Zeile
   gemeinsamen Code. Sie stimmen über den gesamten Vektorsatz byteweise überein
   — 232 Paare — und über fünfzehn Fehlerfälle, was ebenso zählt: wer sich über
   gültige Ströme einig ist und über ungültige nicht, ist sich über das Format
   nicht einig. Über den Vektorsatz hinaus kodieren beide dieselbe
   262923-Byte-Eingabe und schreiben Zeichen für Zeichen denselben Strom
   (`conformance/test_large.py`) — vier Fenstergrenzen, an denen die
   Naht­verschmelzung aus §9.2.1 die einzige Stelle ist, an der zwei
   Implementierungen leise auseinanderlaufen könnten.
   Die Lücke: derselbe Autor. Eine dritte Implementierung von jemand anderem
   prüft sich gegen `docs/vectors.json`, ohne eine der beiden zu lesen.
4. **Dieselbe Entscheidung in §9.6.** Die Entropie ist ganzzahlig zu bestimmen,
   und zwei Implementierungen müssen für dieselbe Stichprobe dieselbe Zahl
   herausbekommen — sonst schreiben sie für dieselbe Eingabe verschiedene
   Bytes, ohne dass ein Testvektor unter 4096 Bytes es merkte.
   **Erbracht:** `conformance/test_classify.py` hält beide Implementierungen
   gegeneinander, auch auf Eingaben, die den Schwellwert genau treffen (7400)
   und um neun Tausendstel verfehlen (7409). Sie entscheiden gleich.

Ergänzende Arbeiten, nicht normativ:

5. Messen (§12, §13): Korpusdichte und Durchsatz über binary2textbench —
   **erbracht**, die Zahlen stehen in §12 und §13. Base65t ist dort als siebter
   Codec eingehängt und wird bei jeder Änderung mitgemessen.
6. Container-Test mit echten Parsern — **erledigt für Pythons Parser**,
   `conformance/test_containers.py`: URL-Query gegen `urllib.parse`, Cookie gegen
   `http.cookies`, JSON gegen `json`, dazu Dateiname und Logzeile. Profil U
   geht durch alle unverändert; Profil T braucht in einer URL Prozent-Encoding
   und enthält das Leerzeichen — beides Negativkontrollen, und die zweite hat
   den Zusatz in §7 hervorgebracht. Ein Parser-Satz, nicht alle: Browser,
   Proxies und Frameworks bleiben offen.
7. API-Form je Zielsprache: `encode` / `decode` analog zum dortigen `base64`;
   zusätzlich `decode_url_strict` und `encode_base64url`, und sonst nichts.
   Rust liegt bei; `python/` ist ein PyO3-Binding darüber und exportiert
   dieselbe Menge, damit ein Python-Aufrufer byteweise dasselbe bekommt wie ein
   Rust-Aufrufer. Ein Binding ist ausdrücklich **keine** zweite Implementierung
   im Sinne von Nachweis 3 — es kann der ersten gar nicht widersprechen.
8. Vektorsatz: `docs/vectors.json` führt 137 Vektoren über beide
   Einstiegspunkte und beide Profile. Der Satz ist kleiner als die 449 der v0.2,
   weil es fünf Presets weniger und ein Profil weniger gibt; die Zahl der
   *Eingaben* ist dieselbe. Der Fuzzing-Korpus für alle zehn Fehlercodes liegt
   in der Testsuite der Referenzimplementierung.

## 17. Erweiterungskandidaten (nicht Teil von v0.4)

1. **Framing.** War in v0.1 und v0.2 Teil des Formats und ist mit v0.4
   gestrichen (§8), weil es die einzige Ausnahme von §9.4 war. `L1 = 0` bleibt
   reserviert, die zwei Zeichen `~A` sind also frei; eine spätere Revision kann
   die Frage neu stellen und müsste dann sagen, was die Ausnahme wert ist.
2. **Profil-Aushandlung.** Aus dem Strom prinzipiell nicht ableitbar (§7.2); ein
   1-char-Präfix wäre selbstbeschreibend, kostet aber ein Zeichen.
3. **Case-insensitive Profil.** Bräuchte einen Base32-Kern — im Grunde ein eigenes
   Format.
4. **Ein verzweigungsfreier Rückwärtslauf.** Kein Formatthema, aber der offene
   Punkt aus §13.3: das Programm ist O(n) und läuft mit 45 bis 69 MiB/s, und
   der Grund sind die unvorhersagbaren Sprünge in den beiden monotonen Deques.
