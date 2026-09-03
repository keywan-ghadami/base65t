# Base65t — Spezifikation v0.2 (final)

> **Historie.** Der aktuelle Stand ist `docs/spec-v0.4.md`. Dieses Dokument
> ist nicht normativ; es liegt hier, weil es trägt, wie es dazu kam.
> `docs/history/README.md` sagt, was zwischen den Fassungen gestrichen wurde
> und warum.

**Status:** final. Ersetzt v0.1; `spec-v0.1.de.md` und `errata-v0.1.de.md`
bleiben als Geschichte im Repository liegen.
**Kurzfassung:** Base64URL, erweitert um ein 65. Zeichen (`~`), das längenpräfigierte
Klartext-Segmente einleitet.

**Am Wire-Format ändert v0.2 nichts.** Jeder v0.1-Strom ist ein v0.2-Strom und
umgekehrt; was sich ändert, ist, welche Segmentierung ein Encoder davon wählen
muss, und an zwei Stellen, was ein Dekoder ablehnt.

> Normative Aussagen sind als solche gekennzeichnet und verwenden MUSS / DARF NICHT /
> SOLLTE nach RFC 2119. Zahlen, die nicht als *exakt* markiert sind, sind Schätzungen
> und mit **[OFFEN]** gekennzeichnet; sie sind nicht normativ und ihre spätere
> Bestimmung ändert das Format nicht.

## Änderungen gegenüber v0.1

Die Abschnittsnummern sind unverändert, damit jede Referenz aus v0.1 weiter
trägt. Die Begründung je Punkt steht in `errata-v0.1.de.md`, die Messungen in
deren Anhang und in `PREREGISTRATION.md`.

| § | Änderung |
|---|---|
| 5.3 | Regel P gilt nur im Plain Mode. „Strom" ist immer der ganze Oktett-Strom, nie ein Frame-Body |
| 5.4 | Regel A gilt über den ganzen Strom, auch über Framegrenzen |
| 9.0 | Bei Längengleichstand ist die Segmentierung vorgeschrieben, nicht freigestellt |
| 2, 13 | Durchsatz ist ein **Ziel**, kein Nicht-Ziel. Was das heißt und was nicht, steht in §13.2 |
| 9.2, 9.2.1 | `dense` und `framed` sind durch eine **lineare Regel** definiert, nicht durch das Programm aus §9.2.2. Ein Durchlauf, konstanter Speicher, und §9.1 zeigt, dass sie §9.4 nicht verletzen kann |
| 9.3 | Die Spalte „Determinismus" wird zu „parameterfrei" |
| 9.3, 15 | **`legible` ist gestrichen.** Sein Tie-Break brauchte eine zweite Kostenkomponente, und deren lexikografischer Vergleich kostete das Programm aus §9.2 zwischen 60 und 190 % — bei *jedem* Preset, auch den vieren, die ihn nie verlangt haben. TV14 ist zurückgezogen |
| 9.4 | Gilt für alle Presets außer `framed` statt für eines. Die Ausnahme ist beziffert |
| 10.1 | Vierte Implementierungsfalle: die Marker-Prüfung aus §10.3 braucht eine Längenprüfung |
| 11, 11.1 | Der Encoder ist jetzt eine Funktion. Die *Berechnung* in §11.1 war falsch und ist korrigiert; ihre O(n)-Zusage gilt der Kostentabelle |
| 12 | Die Binärzeile ist eine Aussage über Profil U und T |
| 15 | TV2, TV5a und TV11 korrigiert; TV13–TV15 neu |
| 16 | Nachweis 3 und 6 sind erbracht, mit benannten Lücken; 5 ist nicht mehr bindend |
| 7 | Profil T enthält das Leerzeichen — der Zusatz, den der Container-Test hervorgebracht hat |
| 9.5, 9.6 | Als Formatfrage geschlossen: eine Messung darf ein Preset nie ändern, nur eines hinzufügen — und genau das ist mit `dense-fast` (§9.6) geschehen |
| 13.2 | Die erfundenen Durchsatz-Schwellwerte sind durch ein gemessenes Kriterium ersetzt |

---

## 0. Positionierung (nicht normativ)

### 0.1 Format und Presets

Base65t ist **ein Format** mit **fünf Presets**. Die Trennung ist wichtig, weil die
Zielkontexte unterschiedliche Optimierungsziele haben.

```
base65t                      # Format: Segmente, Alphabet, Profile, Framing
├── dense       (Default)    # klein und schnell         -> URL, Header, allgemein
├── dense-fast               # dasselbe, ohne zu suchen, wo Suchen nichts bringt
├── canonical                # minimale Größe, byte-identisch -> Cache-Keys, Dedup
├── opaque                   # garantiert keine Literale -> Tokens mit Geheimnisanteil
└── framed                   # wahlfreier Zugriff        -> Storage, Streams
```

| Einsatz | Preset | Profil | worauf es ankommt |
|---------|--------|--------|-------------------|
| URL-Query | `dense` | U | URL-Sicherheit ohne Prozent-Encoding |
| Cookie-Wert | `dense` | U | `cookie-octet`-Konformität (§7.1) |
| HTTP-Header | `dense` | U oder T | ASCII, keine Trennzeichen |
| Token mit Secret | `opaque` | — | keine Klartext-Leaks (§14) |
| Cache-/Dedup-Key | `canonical` | wie Container | byteweiser Determinismus (§11.1), kürzeste Ausgabe |
| Log-Feld | `dense` | T | dort bleibt der Text lesbar (§13) |
| Massendaten, Durchsatz | `dense-fast` | U | so groß wie `dense`, nur wo Suchen sich lohnt (§9.6) |

### 0.2 Was Base65t *ist*, in einem Satz

> Ein segmentiertes Hybrid-Encoding: Base64URL als Binärrepräsentation,
> längenbegrenzte Rohbyte-Literale als zweite Repräsentation, und ein 65. Zeichen
> als Diskriminator zwischen beiden.

### 0.3 Was „ein Dekoder für alles" genau heißt

> Ein konformer `decode()` nimmt einen Octet-Stream und ein Profil entgegen und
> benötigt **keinen weiteren Parameter**. Alphabetvariante (§5.2), Padding (§5.3)
> und Framing (§5.6) werden aus dem Strom selbst bestimmt und im Ergebnis gemeldet.

Das Profil bleibt Parameter, weil es eine Aussage über den *Container* ist, nicht
über den Strom, und aus dem Strom prinzipiell nicht ableitbar (§7.2).

### 0.4 Warum nicht „Base85N mit URL-Alphabet"

Die *unreserved*-Menge von RFC 3986 umfasst 66 Zeichen. Eine Radix-85-Kodierung ist
darin darstellbar, aber ein Passthrough vom Typ Base85N braucht zusätzlich
Spender-Zeichen für die R-Set-Substitution — dafür bleibt kein Spielraum, sobald 64
Zeichen für den Binärkern gebunden sind. Base65t geht den entgegengesetzten Weg: ein
Kern, der **exakt Base64URL ist**, plus ein Diskriminator. Nur daraus folgt die
Superset-Eigenschaft aus §5.2; mit einem Radix-85-Kern wäre sie nicht zu haben.

## 1. Zielsetzung

1. Im `dense` Plain Mode **binär nie schlechter als Base64** (§9.4).
2. Profil-legalen Klartext nahezu verlustfrei durchreichen (≈ 1,001).
3. Lesbar bleiben.
4. **Kein Escaping** — auch nicht für `~`.
5. **Abwärtskompatibel lesen** — jeder kanonische Base64- oder Base64URL-Strom,
   gepaddet oder nicht, dekodiert zu denselben Bytes (§5.2, §5.3). Normativ.
6. **Selbstbestimmend im Strom** — Alphabet, Padding und Framing werden erkannt,
   nicht konfiguriert (§0.3).
7. Optional wahlfrei dekodierbar (§8).
8. Deterministisch reproduzierbar, wo nötig (§11.1).

### 1.1 Die Kompatibilität ist asymmetrisch

| Richtung | Gilt? |
|----------|-------|
| Base65t-Dekoder liest Base64URL, ungepaddet | **ja**, normativ |
| Base65t-Dekoder liest Base64URL, gepaddet | **ja**, normativ, §5.3 |
| Base65t-Dekoder liest klassisches Base64 (`+`/`/`), gepaddet oder nicht | **ja**, normativ, §5.2/§5.3 |
| Base65t-Dekoder erkennt Plain vs. Framed selbst | **ja**, normativ, §5.6 |
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
* **Nicht kanonisch im Default.** §11; dafür gibt es `canonical` (§11.1).
* **Kein Sicherheitsmechanismus.**

## 3. Notation

* `byte` = Byte der Nutzdaten. `char` = Zeichen des Ausgabestroms.
* **Base65t erzeugt einen Octet-Stream.** In Profil U und T ist jedes Oktett
  druckbares ASCII; in Profil B nicht.
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
DARF einen Base64-Lauf deshalb **nicht** aufteilen.

**Literal-Läufe sind dagegen *nicht* automatisch maximal.** Zwei angrenzende
LiteralSegmente tragen je einen eigenen Header und sind vom Dekoder sehr wohl
unterscheidbar; die Grammatik erlaubt sie ausdrücklich. Für den Encoder ist das eine
echte Wahl — und für `canonical` der Grund, warum eine bloße Byte-Klassifikation
nicht ausreicht (§11.1).

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
> abgeschlossen sein. Der Dekoder MUSS diese akzeptieren.
>
> **Strom heißt der ganze Oktett-Strom**, nie ein Frame-Body. Regel P gilt
> deshalb **nur im Plain Mode**: ein `=` innerhalb eines Frames ist kein
> Padding, sondern ein Zeichen ohne Wert an einer Alphabetposition und damit
> `E_CHARSET` (§10.4).

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

**Warum Frames ausgenommen sind.** Padding existiert, damit ein Erzeuger
gewöhnlichen Base64 nichts ändern muss (§1.1). Kein solcher Erzeuger schreibt
Frames. Innerhalb eines Frames wäre es also eine zusätzliche
Parser-Differential-Fläche (§14) ohne Gegenwert — und zwei Dekoder könnten über
denselben Strom verschieden urteilen, während beide dieser Spezifikation
folgen. Siehe TV15.

### 5.4 Alphabet-Konsistenz (Regel A, normativ)

Ohne Zusatzregel hätte ein Strom mit `k` Alphabetzeichen aus {62, 63} genau `2^k`
Schreibweisen derselben Bytes.

> **Regel A.** Ein Strom DARF NICHT beide Alphabetvarianten mischen. Enthält die
> Menge der **Alphabetzeichen** sowohl ein Zeichen aus {`+`,`/`} als auch eines aus
> {`-`,`_`} → `E_MIXED_ALPHABET`. Auch hier ist der Strom der ganze
> Oktett-Strom: im Framed Mode gilt die Regel **über Framegrenzen hinweg**.

Damit sinkt die Ambiguität von `2^k` auf **2**. Kosten: ein Bit Dekoderzustand.

**Wichtig:** Regel A betrifft ausschließlich Alphabetzeichen. Literal-Payloads
zählen nicht mit — in Profil U enthält fast jede Payload `-` oder `_`. Wer den
Gesamtstrom scannt, weist gültige Ströme ab (TV7).

### 5.5 Meldung und strikte Varianten (normativ)

Permissivität darf die Validierung nicht kosten. Ein `decode()`-Ergebnis MUSS
enthalten:

```
alphabet_seen : { none, url, classic }
padding_seen  : bool
framing_seen  : { plain, framed }
```

Zusätzlich MÜSSEN angeboten werden: `decode_url_strict` (weist `classic` mit
`E_NON_URL_ALPHABET` ab), `decode_plain`, `decode_framed` (§5.6). Alle drei Flags
fallen im Parser ohnehin an; die Meldung ist zur Laufzeit gratis.

### 5.6 Framing-Erkennung (normativ)

> **Regel F.** Ein nichtleerer Strom, dessen erste zwei Oktette `~A` sind, ist
> **Framed**. Jeder andere nichtleere Strom ist **Plain**. Der leere Strom ist in
> beiden Modi gültig und dekodiert zu null Bytes.

**Beweis.** Für einen gültigen **Plain**-Strom gilt: entweder ist `stream[0]` ein
Alphabetzeichen — dann beginnt ein Base64-Segment und `stream[0] != '~'` — oder
`stream[0] == '~'`, dann ist `stream[1]` ein Längenzeichen. Der Wert 0 ist an
Längenposition reserviert (§6.1) und führt zu `E_RESERVED_LEN`; das einzige Zeichen
mit Wert 0 ist `A`. Ein gültiger Plain-Strom beginnt also nie mit `~A`.
Für einen **Framed**-Strom gilt nach der Grammatik in §8.1, dass jeder Frame — und
damit der Strom — mit `~A` beginnt. Die beiden Mengen sind disjunkt und decken alle
nichtleeren Ströme ab. ∎

Ein konformer `decode()` MUSS Regel F anwenden und `framing_seen` melden. Die
Erkennung ist O(1) und benötigt keinen Rückgriff.

**Grenzfall leerer Strom.** Für *nichtleere* Ströme ist die Erkennung eindeutig aus
dem Strom bestimmt. Der leere Strom trägt keine Information und ist in beiden Modi
gültig; `decode()` meldet für ihn **konventionsgemäß** `plain`. Der Anspruch aus
§0.3 gilt also präzise für nichtleere Ströme; für den leeren ist er eine Festlegung,
keine Ableitung.

**Sicherheitshinweis.** Auto-Erkennung ist eine Parser-Differential-Fläche: ein
Angreifer, der den Strom kontrolliert, wählt damit den Modus und verändert die
dekodierten Bytes. Anwendungen, die einen festen Modus erwarten, SOLLTEN
`decode_plain` bzw. `decode_framed` verwenden oder auf `framing_seen` prüfen. Regel F
schafft Bequemlichkeit, nicht Vertrauen.

## 6. Literal-Segment

```
LiteralSegment := "~" LengthHeader Payload
```

### 6.1 Längen-Header

| `L1` | Bedeutung |
|------|-----------|
| 0 (`A`) | **Reserviert.** Plain: `E_RESERVED_LEN`. Framed: Frame-Header (§8). Trägt Regel F (§5.6). |
| 1–62 | Länge = `L1`. Header 2 chars. |
| 63 (`_`/`/`) | Erweiterung: nächste zwei Zeichen = 12 Bit `V`. Länge = `63 + V`, Bereich 63–4158. Header 4 chars. |

Encoder MUSS die kürzeste Header-Form wählen. Läufe > 4158 Bytes → mehrere
LiteralSegmente. Dichte reinen Literals: `4162/4158 = 1,00096`.

### 6.2 Payload

Exakt `L` rohe Bytes, unverändert. Kein Zeichen der Payload ist Steuerzeichen — auch
`~` nicht. Kein Escaping, weil es nichts zu escapen gibt.

## 7. Profile

| Profil | Erlaubte Literal-Bytes | Strom | URL-Query direkt? |
|--------|------------------------|-------|-------------------|
| **U** (Default) | RFC-3986-*unreserved* (66 Zeichen) | ASCII | **ja** |
| **T** | ASCII 0x20–0x7E ohne `"` und `\` | ASCII | **nein** |
| **B** | 0x00–0xFF | Oktette | **nein** |

Profilwidrige Payload → `E_PROFILE`. Ein profilwidriges Byte ist kein Sonderfall: es
landet im Base64-Segment.

**Profil T** ist JSON-String-sicher, **nicht** CSV-struktursicher und **nicht**
URL-sicher: `,` `;` `?` `&` `=` `+` `/` `#` sind erlaubt. **Und es enthält das
Leerzeichen** (0x20): eine whitespace-getrennte Logzeile muss einen T-Wert also
quoten, ein `key=value`-Format nicht. Wer eine Logzeile an Leerzeichen
zerlegt, nimmt Profil U — dessen Alphabet enthält keines. Gefunden vom
Container-Test aus §16.6, nicht aus der Tabelle abgelesen.

**Profil B** verlässt die ASCII-Eigenschaft und DARF NICHT in URLs, Cookies, Headern
oder Textcontainern verwendet werden.

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
*unreserved*-Bytes enthalten, ist unter U, T und B identisch gültig. Es beschreibt
die Erwartung des **Containers**, nicht eine Eigenschaft des Stroms. Deshalb ist es
der einzige Parameter, den `decode()` behält (§0.3).

## 8. Framed Mode (Preset `framed`)

Plain Mode ist nicht wahlfrei dekodierbar: rückwärts lässt sich nicht entscheiden, ob
ein `~` Steuerzeichen war oder Literal-Daten aus einem früher begonnenen Lauf. Worst
Case O(n).

Framed kostet 5 chars pro Frame; §9.4 gilt für ihn nicht.

### 8.1 Struktur

```
FramedStream := Frame*
Frame        := "~A" FrameLen FrameBody
FrameLen     := <3 Alphabetzeichen>   # 18 Bit, MSB-first: Länge von FrameBody in chars
FrameBody    := <Plain-Mode-Stream>
```

Max. Frame-Body 262143 chars. **Empfehlung:** jeder Frame außer dem letzten
dekodiert zu exakt 65536 Bytes → Offset-zu-Frame-Index in O(1) ohne Trailer.

### 8.2 Eindeutigkeit des Markers

**Invariante F′ (normativ).** Im Framed Mode DARF die Zeichenfolge `~A` ausschließlich
als Frame-Header auftreten.

Zwei hinreichende, encoderseitig prüfbare Regeln je Literal-Payload:

* **F1:** Die Payload DARF NICHT die 2-Byte-Folge `~A` enthalten.
* **F2:** Das letzte Byte der Payload DARF NICHT `~` sein.

**Durchsetzung.** Teilen des Laufs erfüllt F1/F2 **nicht**. Bei `hello~Alice`:

| Schnitt | Ergebnis | Problem |
|---------|----------|---------|
| vor der Tilde | `hello` + `~Alice` | zweites Literal enthält `~A` → F1 |
| zwischen `~` und `A` | `hello~` + `Alice` | erstes Literal endet auf `~` → F2 |

Der Encoder MUSS das `~`-Byte in ein Base64-Segment auslagern:

```
Literal("hello") -> Base64(0x7E) -> Literal("Alice")
Strom:  ~Fhellofg~FAlice
```

Kosten ca. 4–5 chars pro Vorkommnis; die DP-Kostenfunktion MUSS das abbilden.

**Beweis.** Ein `~` hat genau vier mögliche Ursachen: (1) Segment-Einleiter — dann
ist das Folgezeichen ein Längenzeichen, und Wert 0 ist als Literal-Länge ungültig,
also per Definition ein Frame-Header; (2) Literal-Datenbyte, nicht letztes — nach F1
folgt kein `A`; (3) Literal-Datenbyte, letztes — nach F2 ausgeschlossen; (4)
Base64-Zeichen — unmöglich. ∎

**Dekoder-Anforderung.** Der Dekoder prüft **ausschließlich F′** (`E_FRAME_RULE`).
F2 ist strenger als F′: ein auf `~` endendes Literal ist nur schädlich, wenn das
Folgesegment mit `A` beginnt. Ein F2-Verstoß, der F′ wahrt, MUSS akzeptiert werden
(TV9).

## 9. Encoder

### 9.0 Grundprinzip (normativ)

> Wo ein Preset optimiert, optimiert es über die Menge der **im jeweiligen
> Modus gültigen** Segmentierungen — nicht über alle denkbaren.

**Jedes Preset ist eine Funktion (normativ).**

> Die Ausgabe eines Presets MUSS durch (Eingabe, Preset, Profil) eindeutig
> bestimmt sein. Ein Preset, dessen Regel mehrere Segmentierungen zulässt, ist
> keine Spezifikation, sondern eine Einladung.

Es gibt zwei Wege dahin, und Base65t geht beide:

* **Eine Regel, die nur eine Segmentierung erzeugt.** So sind `dense` und
  `framed` definiert (§9.2.1): ein Vorwärtsdurchlauf ohne Wahlfreiheit.
* **Eine Zielfunktion plus eine Ordnung für Gleichstände.** So sind
  `canonical` und `opaque` definiert. Für sie gilt:

> Haben mehrere gültige Segmentierungen dieselbe Länge, MUSS ein Encoder die
> nach der Ordnung aus §11.1 kleinste wählen.

Damit kann ein Testvektor Bytes prüfen statt nur Längen — was §16.8 braucht und
was `docs/vectors.json` über 449 Vektoren tut.

### 9.1 Schwellwert

Ein Literal spart `L/3` chars, kostet 2 chars Header plus Rundungsverschnitt. Mit
`r(B) = ceil(4B/3) − 4B/3 ∈ {0, ⅓, ⅔}` und maximaler Zusatzrundung `4/3`:

```
Ersparnis_worst(L) = L/3 − 2 − 4/3 = (L − 10)/3
```

`L ≤ 9` negativ, `L = 10` neutral, `L ≥ 11` immer ein Gewinn.
**Normativ für `dense` und `framed`:** Literale nur ab `L ≥ 11`.

**Das ist mehr als ein Schwellwert — es ist der Grund, warum §9.4 ohne
Optimierung gilt.** Die Rechnung oben lädt einem einzelnen Literal bereits die
schlechteste Rundung auf beiden Seiten auf. Ein Encoder, der *nur* Literale ab
11 Bytes nimmt, kann deshalb nicht verlieren, gleichgültig welche er nimmt und
wie viele. Für `k ≥ 1` Literale mit Header `h_j ∈ {2, 4}` und Längen `L_j`
(`L_j ≥ 11` bei `h_j = 2`, `L_j ≥ 63` bei `h_j = 4`) gilt gegenüber reinem
Base64:

```
Differenz  ≤  Σ_j (h_j − L_j/3)  +  2(k+1)/3  ≤  −k + 2/3  <  0
```

Der zweite Term ist die Rundung der höchstens `k + 1` Base64-Läufe, der erste
je Literal höchstens `2 − 11/3 = −5/3`. Deshalb braucht §9.4 **keine
Optimalität**, sondern nur den Schwellwert — und deshalb darf §9.2.1 eine
lineare Regel sein.

### 9.2 Optimale Segmentierung — Herleitung

Dieser Abschnitt leitet die **längenoptimale** Segmentierung her. Sie ist die
Definition von `canonical` und `opaque`. `dense` und `framed`
benutzen sie **nicht**; sie sind in §9.2.1 durch eine lineare Regel definiert,
und §9.2.2 fasst zusammen, was sie dafür aufgeben und was nicht.

**Literale werden nicht als Zustand, sondern als Kante modelliert.**

**Definitionen.** `D[j]` = minimale Kosten, die Bytes `[0, j)` so zu kodieren, dass
bei `j` eine Segmentgrenze liegt. `B[j][p]` = minimale Kosten für `[0, j)` mit
offenem Base64-Segment, `p ∈ {0,1,2}` Bytes im angebrochenen Quantum.

**Base64-Kanten** sind O(1): die Zeichenkosten je Byte hängen nur von `p` ab
(`p=0→1`: +2 chars, `p=1→2`: +1, `p=2→0`: +1; Summe 4 chars je 3 Bytes ✓). Ein
Base64-Segment darf bei jedem `p` enden, also `D[j] ← min_p B[j][p]`.

**Literal-Kanten.** Ein Literal von `i` nach `j` mit `m = j − i` kostet
`m + h(m)`, wobei

```
h(m) = 2   für 1 ≤ m ≤ 62
h(m) = 4   für 63 ≤ m ≤ 4158
```

Läufe > 4158 entstehen als **mehrere** Kanten, brauchen also keinen dritten Fall.
Damit hat `h` genau **zwei Bänder**, und:

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
ausdrücken lassen, damit die Deques sie tragen.

* **Profil.** Eine Literal-Kante `[i, j)` verlangt durchgehend profil-legale Bytes.
  Sei `bad(j)` die letzte profilwidrige Position vor `j`; gültiges Fenster ist
  `i > bad(j)`. Umgesetzt durch Leeren beider Deques beim Passieren eines
  profilwidrigen Bytes — O(1) amortisiert.
* **F1 (Framed).** Die Bedingung ist, dass **innerhalb** `[i, j)` keine Position mit
  `byte[q] == 0x7E ∧ byte[q+1] == 'A'` liegt — strukturell dieselbe Bedingung wie
  beim Profil. Sei `tildeA(j)` die letzte solche Position `q < j`; gültiges Fenster
  ist `i > tildeA(j)`. Beide Schranken verschmelzen zu `i > max(bad(j), tildeA(j))`
  und werden durch dasselbe Deque-Leeren umgesetzt — O(1) amortisiert.
* **F2 (Framed).** Verbietet Kanten, deren **letztes** Byte `0x7E` ist. Das ist eine
  Eigenschaft von `j` allein, nicht der Kante: bei `byte[j-1] == 0x7E` entfällt die
  Literal-Transition nach `j` ersatzlos — O(1) pro Position.

Damit sind alle Bedingungen Fenster- oder Positionsbedingungen, und der O(n)-Beweis
trägt auch im Framed Mode.

**Speicher.** Kostenberechnung O(1) zusätzlich (zwei Deques der Breite 62 bzw. 4096
→ O(4158) konstant). **Rekonstruktion** der Segmentierung O(n) Backpointer.

### 9.2.1 Die lineare Regel (normativ für `dense` und `framed`)

> **Regel.** Setze `p = 0` (Anfang des noch offenen Base64-Laufs) und `i = 0`.
> Solange `i < n`:
>
> 1. Sei `j` die größte Position mit `j − i ≤ 4158`, so dass alle Bytes in
>    `[i, j)` profil-legal sind; im Framed Mode zusätzlich so, dass `[i, j)`
>    kein `~A` enthält (F1) und nicht auf `~` endet (F2).
> 2. Ist `j − i ≥ 11`: schließe einen Base64-Lauf `[p, i)`, falls `p < i`,
>    schreibe das Literal `[i, j)`, setze `i = p = j`.
> 3. Sonst: setze `i = max(j, i + 1)`.
>
> Am Ende schließe `[p, n)`, falls `p < n`.

Ein Vorwärtsdurchlauf, keine Rückwärtskosten, keine Backpointer: **O(n) Zeit,
O(1) Speicher**, streamfähig ohne jede Zusatzkonstruktion. Die Ausgabe ist eine
Funktion der Eingabe, denn die Regel trifft keine Wahl — Schritt 1 bestimmt `j`
eindeutig.

**Warum das erlaubt ist.** §9.1 zeigt, dass ein Literal ab 11 Bytes nach der
schlechtesten Rundung auf beiden Seiten nicht verlieren kann. Die Regel nimmt
nur solche Literale. Also gilt §9.4 für sie — nicht weil sie optimiert, sondern
weil sie unter der Schwelle nichts anfasst. Eine Optimierung ist für die
Nie-schlechter-Garantie gar nicht nötig; v0.2 hat das eine Zeit lang anders
gesehen und dafür den Encoder in Blöcke geschnitten, die es jetzt nicht mehr
gibt.

**Was sie kostet.** Die Regel ist nicht längenoptimal: sie absorbiert nie ein
Byte in einen Base64-Lauf, um ein Quantum auszurichten, und sie beendet ein
Literal nie früh, um denselben Effekt zu erzielen. Gemessen über 110 Dateien
einschließlich Silesia (202 MiB), gegen `canonical` auf denselben Eingaben:

| | `dense` | `canonical` |
|---|---|---|
| Summe über den Korpus | 99,160 % von Base64 | 98,938 % |
| Abstand zueinander | **+0,224 %** | — |
| schlechteste Einzeldatei | **+4,545 %** (eine 22-Byte-Telefonnummer) | — |

Der Worst Case ist eine kurze Eingabe, auf der ein einzelnes ausgerichtetes
Quantum prozentual viel wiegt; über alles, was länger als ein paar Dutzend
Bytes ist, verschwindet der Unterschied. Wer ihn trotzdem nicht will, nimmt
`canonical` — das ist der Unterschied, für den es das Preset gibt. Der Abstand
nach *unten* zu Base64 bleibt in beiden Fällen (§9.4).

**Was sie bringt.** Auf demselben Korpus, gegen dieselben Eingaben:

| | `dense` alt (§9.2.2 über Blöcke) | `dense` neu (lineare Regel) |
|---|---|---|
| Kodieren | 1478 % der Base64-Zeit | **114 %** |
| Dekodieren | 678 % | **107 %** |
| Größe | 131,9 % der Eingabe | 132,0 % |

Ein Zehntelpunkt Dichte gegen den Faktor 12 beim Kodieren. Die Richtung dieses
Tauschs ist §2 und §13.2: Größe darf nie schlechter werden als Base64, und
innerhalb dieser Schranke zählt Durchsatz.

**Was der Sprung nicht kann.** Er überspringt Fenster, in denen ein Byte die
Profilmenge verlässt. Text, dessen profilwidrige Zeichen dicht stehen — Prosa
in Profil U, wo alle fünf Zeichen ein Leerzeichen steht —, hat solche Fenster
kaum: dort liest der Encoder jedes Byte, verwirft jeden Lauf und schreibt reines
Base64. §13 beziffert das; §9.5 sagt, was man dagegen täte und warum es ein
eigenes Preset wäre.

**Implementierungshinweis (nicht normativ).** Schritt 3 darf springen: Ein
Literal von 11 Bytes, das irgendwo in `[i, i + 11)` beginnt, überdeckt
notwendig `i + 10`. Ist `byte[i + 10]` nicht profil-legal, kommt kein Start in
diesem Fenster in Frage, und `i` darf sofort auf `i + 11` springen. Auf
hochentropen Daten — wo fast jedes Byte die Profilmenge verlässt — ist das der
Unterschied zwischen einer Prüfung je Byte und einer je elf. Die gefundenen
Literale sind dieselben: die Bedingung ist notwendig, nie hinreichend.

### 9.2.1.1 Parallelisierung (nicht normativ, aber eine Aussage über das Format)

Die lineare Regel ist parallelisierbar, **ohne dass sich ein Byte der Ausgabe
ändert**. Das ist keine Implementierungsfreiheit, sondern folgt aus zwei
Eigenschaften des Formats:

1. **Ein profilwidriges Byte liegt in keinem Literal.** Ein Encoder, der bei
   einem solchen Byte ansetzt, hat also nichts Offenes über diese Stelle hinweg
   und entscheidet ab dort genau das, was ein Encoder entscheidet, der am
   Anfang der Eingabe begonnen hat. Die Regel ist gedächtnislos in `i`.
2. **Ein Base64-Lauf überschreitet nie ein Literal.** Ein Schnitt auf dem
   *ersten Byte eines Literals* lässt den Lauf davor vollständig auf der einen
   Seite; die beiden Hälften ergeben aneinandergehängt genau den Strom, den ein
   Durchlauf über die ganze Eingabe geschrieben hätte.

> Ein Encoder DARF die Eingabe an Schnittstellen zerlegen, die nach 1. gefunden
> und nach 2. gewählt sind, und die Teile unabhängig kodieren. Die Ausgabe MUSS
> dieselbe sein wie die eines einzelnen Durchlaufs — die Zahl der Arbeiter ist
> ein Geschwindigkeitsregler und darf nie im Strom sichtbar werden. §11.1 hängt
> Cache-Keys an diese Bytes; ein Strom, der von der Kernzahl der Maschine
> abhinge, wäre auf jeder Maschine ein anderes Format.

Einen Schnittpunkt findet man **lokal**: ab der Zielstelle das nächste
profilwidrige Byte suchen, von dort die Regel laufen lassen und das erste
Literal nehmen, das sie nimmt. Ein Fenster genügt — ein Literal, das klar vor
dem Fensterende endet, wurde aus Bytes innerhalb des Fensters entschieden.

Wo die Regel kein Literal findet, gibt es keinen Schnittpunkt. Das kostet
nichts: eine Eingabe ohne Literale ist nach §9.4 byteweise Base64URL, und die
schreibt derselbe Encoder ohnehin mit Base64-Geschwindigkeit.

**`framed` braucht das nicht.** Frames sind selbstbegrenzend (§8.1), also
sowohl parallel kodierbar als auch — anders als der Plain Mode — parallel
**dekodierbar**. Im Plain Mode ist Dekodieren zwangsläufig sequentiell: ob ein
`~` ein Segment einleitet oder Nutzlast ist, sagt einem nur der Parser, der
davor war. Wer wahlfreien Zugriff oder parallele Dekodierung braucht, nimmt
`framed`; genau dafür ist es da.

### 9.2.1.2 Die Maske (nicht normativ)

Die Regel aus §9.2.1 sucht Läufe profil-legaler Bytes von mindestens `L_min`
Länge. Byteweise gelesen ist die Frage „geht dieser Lauf noch weiter" eine
datenabhängige Verzweigung, und für gemischte Daten ist sie nicht vorhersagbar:
je Lauf eine Fehlvorhersage. Auf Prosa in Profil U, wo alle fünf Zeichen ein
Leerzeichen steht, kostete das mehr als das Base64-Schreiben selbst.

Verzweigungsfrei geht es so:

1. **64 Bytes zu 64 Bits.** Ein Bit je Byte, gesetzt, wenn das Profil es
   zulässt. Keine Verzweigung, ein Tabellenzugriff je Byte.
2. **Läufe ab `k` finden, durch Verdoppeln.** `m &= m >> 1` lässt die Bits
   stehen, hinter denen zwei gesetzte folgen; `m &= m >> 2` macht vier daraus,
   dann acht, dann elf. Vier Schritte, und `m` trägt danach genau dort ein Bit,
   wo ein Lauf von 11 beginnt.
3. **Ist das Wort danach null, sind 64 Bytes erledigt.** Das ist der Normalfall
   auf komprimierten oder verschlüsselten Daten.
4. **Lauflängen kommen aus dem Wort**, nicht aus dem Datenstrom: `trailing_ones`
   ab der Startposition.

Ein Lauf kann eine Blockgrenze überschreiten, deshalb wird über *zwei* Blöcke
gerechnet — ein Start im ersten wird dann an den Bytes gemessen, die ihm
tatsächlich folgen.

Gemessen: der Scan läuft mit 1352 MiB/s **unabhängig von den Daten**, wo er
byteweise zwischen 473 (Prosa) und 1594 (Binär) schwankte. Über acht Megabyte
Prosa heißt das 205 % → 113 % der Base64-Zeit; auf einem 64-Byte-Digest 73 % →
58 %. Die Spanne über alle Datenarten fällt damit von 103–205 % auf 108–174 %.

Das ist eine Implementierungsfrage und keine Formatfrage: die gefundene
Segmentierung ist dieselbe, byteweise wie maskiert, und `tests/linear_rule.rs`
verlangt genau das — es schreibt §9.2.1 ein Byte nach dem anderen ab und
fordert Übereinstimmung über alle Profile, Schwellwerte und Framing-Modi.

**Was hier nicht steht.** SIMD. Der nächste Schritt wäre, Schritt 1 mit
Vektorvergleichen statt mit einer Tabelle zu machen (32 oder 64 Bytes je
Befehlsgruppe statt einem) und das Base64-Schreiben selbst zu vektorisieren
(§13.1). Beides ist bekannte Technik und beides ist zielabhängig; die
Referenzimplementierung bleibt skalar und lesbar (§13, §14).

### 9.2.2 Warum `dense` nicht das Programm aus §9.2 benutzt

Der DP aus §9.2 ist O(n) in der Zeit, aber die Rekonstruktion braucht O(n)
Backpointer, und die Konstanten sind zwei monotone Deques breit. Für ein
Gigabyte-Objekt ist das der Unterschied zwischen „läuft" und „läuft nicht"; für
alles andere ist es der Unterschied zwischen 124 % und 1478 % der Base64-Zeit.

Der Ausweg, den v0.2 zuerst nahm, war Blockbildung: den DP je 65535 Bytes
laufen lassen. Das löst den Speicher und nichts sonst. Es kostet den Durchsatz
weiter, es setzt eine Blockgröße als Parameter in ein Preset, das keinen haben
sollte, und es bringt eine eigene Fehlerklasse mit — ein Block, dessen letzter
Base64-Lauf ein Quantum offen lässt, wird vom nächsten Block fortgesetzt, und
die Naht dekodiert zu etwas, das keiner der beiden Blöcke gemeint hat (§4).
Genau dieser Fehler ist in der Bench aufgetreten, nicht in den Testvektoren:
die reichen nicht über einen Block hinaus.

**Blöcke gibt es in v0.2 nicht.** Die lineare Regel ist streamfähig, ohne dass
der Strom geschnitten werden müsste, und damit ist die Nahtstelle weg statt
abgesichert.

`framed` schneidet weiter in Frames à 65536 Bytes, aber aus einem anderen
Grund: dort ist die Zweierpotenz der Zweck (§8.1, Offset → Frame in O(1)), und
`framed` ist von §9.4 ohnehin ausgenommen. Ein Frame trägt seine Länge im
Header, also gibt es dort keine offene Naht.

### 9.3 Presets

| Preset | Zielfunktion | Framing | Alphabet | parameterfrei |
|--------|--------------|---------|----------|---------------|
| `dense` (Default) | die lineare Regel, Literale ab `L ≥ 11` (§9.2.1) | Plain | URL | nein |
| `dense-fast` | wie `dense`, aber nur in Fenstern, die eine Stichprobe behält (§9.6) | Plain | URL | nein |
| `canonical` | kürzeste, darunter die kleinste nach §11.1 | Plain | URL | **ja** |
| `opaque` | nie Literale (= Base64URL) | Plain | URL | **ja** |
| `framed` | wie `dense`, Frames à 65536 (§8.1) | Framed | URL | nein |

Ein Aufruf ohne Preset MUSS `dense` + Profil U liefern. Bibliotheken SOLLTEN genau
eine parameterlose `encode`-Funktion exportieren.

**Alle fünf sind deterministisch** — das folgt aus §9.0 und ist kein
Unterscheidungsmerkmal mehr. Was sie trennt, ist die letzte Spalte: `dense` und
`framed` tragen Parameter (`L ≥ 11`, Framegröße), die §9.5 noch bewegen könnte;
ihre Ausgabe ist deterministisch, **solange diese Parameter stehen**.
`canonical` und `opaque` haben keine, sind also eingefroren. Deshalb
gehören Cache-Keys an `canonical` und nicht an `dense` (§11.1).

**`dense` ist nicht die kürzeste Ausgabe, `canonical` ist es.** Das ist neu in
v0.2 und die einzige Stelle, an der ein Preset etwas aufgibt: `dense` tauscht
0,2 % Dichte über den Korpus gegen den Faktor 12 im Durchsatz (§9.2.1). Wer die
kürzeste Ausgabe braucht und die Eingabe klein ist — ein Cache-Key, ein
Log-Feld, eine URL —, nimmt `canonical`. Die Nie-schlechter-Garantie aus §9.4
gilt für beide.

**`legible` gab es einmal**, mit derselben Zielfunktion wie `canonical` und
einem Tie-Break zugunsten des Klartexts. Es ist gestrichen. Der Grund ist nicht
sein Nutzen — fünf Punkte mehr Klartext bei gleicher Länge —, sondern sein Preis
an einer Stelle, an der ihn niemand vermutet hätte: der Tie-Break brauchte eine
zweite Kostenkomponente, und die machte aus jedem Kostenvergleich im Programm
aus §9.2 einen lexikografischen, also verzweigten. Das kostete **60 bis 190 %
der Zeit dieses Programms — bei jedem Preset**, auch bei den vieren, die den
Tie-Break nie benutzt haben. Wer Lesbarkeit will, nimmt Profil T: dasselbe XML
kommt dort mit 93 % Klartext heraus statt mit 12 %, und das mit der billigen
Regel aus §9.2.1 (§13).

### 9.4 Nie-schlechter-Garantie (normativ)

Für `dense`, `dense-fast`, `canonical` und `opaque` MUSS gelten:

```
len(encode(x)) <= ceil(4 * len(x) / 3)
```

**Je Eingabe, nicht im Mittel**, und aus zwei verschiedenen Gründen:

* Für `canonical` und `opaque` folgt sie aus §9.0: die reine
  Base64-Segmentierung liegt immer in der Kandidatenmenge, und alle drei
  minimieren die Länge über diese Menge — sie können also nichts Längeres
  wählen.
* Für `dense` folgt sie aus §9.1: die lineare Regel nimmt nur Literale ab 11
  Bytes, und die dortige Rechnung zeigt, dass die schon einzeln nicht verlieren
  können, unabhängig davon, wie viele es sind und wo sie liegen. `dense`
  optimiert nichts und hält die Garantie trotzdem.
* Für `dense-fast` folgt sie aus beidem: ein übersprungenes Fenster ist
  **exakt Base64**, ein durchsuchtes gehorcht §9.1. Deshalb kann eine falsche
  Stichprobe Größe kosten und nie die Garantie (§9.6).

**Schärfer, und der eigentliche Grund für die Umstellung:** auf hochentropen
Daten findet kein Literal einen Platz, und `dense` schreibt dann nicht nur
gleich viele Zeichen wie Base64URL, sondern **dieselben Bytes**. Wo Base65t
nichts holt, ist es Base64 — auch im Durchsatz, weil es derselbe Strom ist.

**Ausnahme `framed`, beziffert.** Ein Frame-Header kostet 5 Zeichen, die Base64
nicht ausgibt; dazu rundet jeder Frame einzeln, weil 65536 nicht durch 3 teilbar
ist. Es gilt

```
len(encode_framed(x)) <= Σ_Frames ceil(4 * len(Frame) / 3) + 5 * Anzahl(Frames)
```

Bei den empfohlenen 65536-Byte-Frames sind das +0,006 %; bei einer 11-Byte-Nutzlast
+33 %. `framed` ist für wahlfreien Zugriff da, nicht für kleine Werte.

**Geltungsbereich.** Die Garantie bezieht sich auf die Länge des kodierten Stroms in
Oktetten, nicht auf Transport- oder Container-Overhead. Prozent-Encoding,
Header-Faltung, Cookie-Attribute oder das Framing eines übergeordneten Protokolls
sind nicht eingerechnet.

### 9.5 Segmentwechselrate **[OFFEN als Messung, geschlossen als Formatfrage]**

Der Durchsatz hängt an datenabhängigen Verzweigungen, also an Segmentwechseln.

**Was exakt gilt** — eine Aussage über *Segmentierungen*, nicht über Durchsatz. Für
eine Segmentierung, in der jeder Literal-Lauf ≥ `L_min` Bytes und jeder
Base64-Lauf ≥ `B_min` Bytes umfasst (beides **in Bytes**, nicht in chars):

```
Segmentwechsel  ≤  2 pro (L_min + B_min) Eingabebytes
```

Das ist reine Kombinatorik über den Eingabestrom.

**Was nicht gilt.** Daraus folgt **kein** Durchsatzmodell. Ein Base64-Lauf von 1 Byte
ist ein vollständiges Segment und erzeugt 2 Ausgabezeichen; die Kosten eines
Segmentwechsels hängen an Pipeline-Tiefe, Sprungvorhersage und Ausgabelänge, nicht
allein an der Eingabebyte-Rate.

**Dass der Worst Case erreichbar ist**, zeigt dieses Beispiel in Profil U:

```
[11 profil-legale Bytes][1 profilwidriges Byte][11 profil-legale Bytes]
DP-Segmentierung:  ~L·11 + b64(1)=2 chars + ~L·11  = 28 chars
Reines Base64 (23 B): ceil(92/3)                    = 31 chars
```

Der DP wählt korrekt 28 chars und erzeugt 2 Wechsel je 12 Bytes. Ohne `B_min` ist
die Wechselrate also nicht durch `L_min` allein gedeckelt — das ist der ganze
Befund, mehr nicht.

**Einordnung.** Der Fall ist eng: er verlangt profil-legale Läufe ≥ `L_min`, getrennt
durch einzelne profilwidrige Bytes. Bei Fließtext in Profil U (Leerzeichen alle ~5
Zeichen) greift der Schwellwert und alles wird ein einziges Base64-Segment.

**Zu entscheiden — durch Messung, nicht durch Formel:**

1. `L_min` — dichteoptimal 11; Kandidaten 16, 32.
2. `B_min` — Mindestlänge eines Base64-Laufs in Bytes, erzwungen durch Absorption
   benachbarter Literal-Bytes. Kandidaten 1 (aus), 4, 8.

Ergebnis ist die Fläche `(L_min, B_min) → (Dichte, Durchsatz)` über den Korpus.

### Was die Messung ändern darf (normativ)

> Eine Messung DARF **kein bestehendes Preset ändern**. Ergibt sie, dass ein
> anderes `L_min` oder ein `B_min > 1` den Durchsatz lohnend verbessert, wird
> daraus ein **neues** Preset (etwa `dense-fast`), nie eine neue Fassung von
> `dense`.

Genau das ist eingetreten, mit einem anderen Hebel als hier erwartet: nicht
`L_min` und nicht `B_min`, sondern die Frage, ob überhaupt gesucht wird. §9.6
ist das Preset, das diese Regel vorsieht, und es heißt wie hier vermutet.

Damit ist §9.5 als Formatfrage geschlossen, ohne dass die Messung ihren Wert
verliert. Der Grund ist nicht Bequemlichkeit: `docs/vectors.json` führt 449
byte-exakte Vektoren, und Cache-Keys, Dedup-Keys und Content-Adressen hängen an
genau diesen Bytes. Ein Preset, das sich später bewegt, bricht sie still.

Für `dense` heißt das konkret: `L_min` bleibt bei **11**, hergeleitet in §9.1
und nicht gemessen, und `B_min` bleibt **aus**. Ein `B_min > 1` erkauft
Durchsatz mit Größe. Durchsatz ist zwar ein Ziel (§13.2), aber Größe ist in §9.4
normativ, und ein Tausch in diese Richtung braucht einen Beleg — das Ergebnis
ist dann ein eigenes Preset. Damit sind **alle Presets eingefroren**, und die Spalte in §9.3
sagt nur noch, ob die Definition Parameter *enthält*, nicht ob sie sich noch
bewegen kann.

### 9.6 `dense-fast`: nicht hinschauen, wo Hinschauen nichts bringt (normativ)

§9.2.1 muss die Eingabe lesen, um zu erfahren, ob ein Literal darin steckt. Wo
keines steckt — und das ist alles, was ein Kompressor liefert —, ist dieses
Lesen Arbeit ohne Gegenwert. Gemessen kostet es rund die Hälfte der Kodierzeit.

> **Regel.** Die Eingabe wird in **Fenster von 65536 Bytes** geschnitten, an
> absoluten Offsets ab Eingabeanfang. Für jedes Fenster ist die **Stichprobe**
> seine ersten **1024 Bytes** (oder das ganze Fenster, wenn es kürzer ist).
> Ein Encoder wendet §9.2.1 auf die Stichprobe an; gehen dabei **weniger als
> ein Zehntel** ihrer Bytes in Literale, MUSS das ganze Fenster ohne Scan als
> Base64 geschrieben werden. Andernfalls gilt §9.2.1 für das Fenster
> unverändert. Ein Literal DARF nur in einem behaltenen Fenster **beginnen**;
> einmal begonnen, läuft es bis zum Ende seines Laufs, über Fenstergrenzen
> hinweg.

**Die Ausgabe bleibt eine Funktion der Eingabe.** Fenstergrenzen liegen an
absoluten Offsets, die Stichprobe ist ein fester Präfix, die Schwelle ist eine
Zahl — es gibt nichts zu raten und nichts, was von der Aufrufreihenfolge
abhinge. §9.0 gilt also unverändert, `docs/vectors.json` führt auch für dieses
Preset byte-exakte Vektoren, und die Aufteilung aus §9.2.1.1 bleibt gültig.

**Eine falsche Entscheidung kostet Größe, nie Korrektheit.** Ein
übersprungenes Fenster ist exakt Base64URL, also greift §9.4 in jedem Fall. Das
ist die Eigenschaft, die es erlaubt, hier überhaupt zu raten — dieselbe
Begründung, die base91z in seinem §11.5 für dieselbe Entscheidung gibt.

**Je Fenster und nicht je Strom**, aus dem Grund, den base91z ebenfalls nennt:
ein `tar` wechselt alle paar hundert Bytes zwischen Textkopf, komprimiertem
Element und Nullpolsterung, und eine Entscheidung am Anfang wäre für den
größten Teil falsch.

**Was es bringt und kostet**, über den Korpus, gegen `dense`:

| Datei | Größe `dense` | Größe `dense-fast` | Kodieren |
|---|--:|--:|--:|
| `random.bin` | 100,0 % | 100,0 % | **1,82×** |
| `dickens` (Silesia) | 99,5 % | 100,0 % | **1,82×** |
| `countries.json` | 99,6 % | 100,0 % | **1,77×** |
| `mozilla` (Silesia) | 98,5 % | 99,2 % | **1,60×** |
| `webster` (Silesia) | 99,6 % | 100,0 % | **1,47×** |
| `requests-2.32.3.tar` | 96,9 % | 98,2 % | **1,26×** |
| `bootstrap.css` | 93,2 % | 93,2 % | 0,94× |

Die letzte Zeile ist die wichtige: wo wirklich Dichte zu holen ist, behält die
Stichprobe jedes Fenster, und dann kostet sie ihre eigenen sechs Prozent. Die
Entscheidung wählt sich selbst aus.

**Zum Zehntel.** Es ist gemessen, nicht hergeleitet. Ein Literal-Byte spart ein
Drittel Zeichen gegen die vier Drittel, die Base64 dafür ausgibt, ein Zehntel
Literalanteil ist also rund 2,5 % der Ausgabe — aber welcher Anteil den Scan
lohnt, hängt daran, was der Scan kostet, und das ist eine Maschineneigenschaft.
Über den Korpus liegt das Knie bei einem Zehntel: bei einem Zwanzigstel fällt
der Gewinn auf 1,4×, bei einem Fünftel fängt `bootstrap.css` an, Dichte zu
verlieren.

**Mit einem vektorisierten Base64-Schreiber zusammen** (§13.1.1) ist das der
Punkt, an dem der Abstand verschwindet: auf `random.bin` schreibt `dense-fast`
dieselben Bytes wie Base64 in 105 % der Zeit, auf `countries.json` in 114 %, wo
`dense` bei 565 % und 325 % liegt. Wer nicht sucht, muss nicht suchen.

## 10. Dekoder

### 10.1 Plain Mode

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
in Profil T fälschlich als Padding gelesen wird (TV10). **(4)** Der Vergleich
`stream[pos..pos+2] == "~A"` in §10.3 steht **vor** der Prüfung `pos + 5 <= len`.
Die Reihenfolge ist richtig — ein nicht gerahmter Strom soll das sagen und nicht
„abgeschnitten" —, aber der Vergleich selbst MUSS längensicher sein: bei
`pos = len − 1` liest eine wörtliche Umsetzung über das Ende hinaus.

### 10.2 Einstiegspunkt

```
decode(stream, profile):
    if len(stream) >= 2 and stream[0..2] == "~A":            # Regel F, §5.6
        framing_seen := framed ; return decode_framed(stream, profile)
    else:
        framing_seen := plain  ; return decode_plain(stream, profile)
```

`decode_plain` und `decode_framed` sind zusätzlich einzeln exportiert und weisen den
jeweils anderen Modus ab (`E_FRAME_SYNC` bzw. `E_RESERVED_LEN`).

### 10.3 Framed Mode

```
while pos < len:
    prüfe: stream[pos..pos+2] == "~A"                    sonst E_FRAME_SYNC
    prüfe: pos + 5 <= len                                sonst E_TRUNCATED
    prüfe: stream[pos+2..pos+5] Alphabetzeichen          sonst E_CHARSET
    FrameLen := 18-Bit-Wert aus stream[pos+2 .. pos+5]
    prüfe: pos + 5 + FrameLen <= len                     sonst E_TRUNCATED
    Body := stream[pos+5 .. pos+5+FrameLen]
    prüfe: Body enthält nirgends die Folge "~A"          sonst E_FRAME_RULE   (F′)
    decode_plain(Body, profile, padding=verboten)   # NICHT decode(), §5.3
    pos += 5 + FrameLen
```

Im Body ist Padding nicht erlaubt (§5.3): der Body ist nicht der Strom, sondern
ein Teil davon. Der Body MUSS mit `decode_plain` verarbeitet werden, **nicht**
mit `decode`. Sonst
liefe Regel F (§5.6) rekursiv auf dem Body, und ein Body, der mit `~A` beginnt,
wechselte erneut in den Framed Mode. Da F′ ein `~A` im Body ausschließt, ist der Fall
unerreichbar — aber nur, solange die F′-Prüfung *vor* der Dekodierung läuft. Die
Reihenfolge oben ist deshalb normativ, und ein `decode()`-Aufruf an dieser Stelle ist
genau die Sorte Parser-Differential, gegen die §14 sonst argumentiert.

Wiedereinstieg an beliebiger Position: vorwärts nach `~A` scannen. Korrektheit
aus §8.2.

### 10.4 Fehlerfälle

| Code | Bedingung |
|------|-----------|
| `E_TRAILING_TILDE` | Strom endet mit `~` oder unvollständigem Header |
| `E_RESERVED_LEN` | `L1 == 0` im Plain Mode |
| `E_TRUNCATED` | Payload oder Frame reicht über das Stromende hinaus |
| `E_PROFILE` | Literal-Byte außerhalb des Profil-Alphabets |
| `E_ALIGN` | Base64-Segmentlänge `mod 4 == 1` |
| `E_NONZERO_TAIL` | Restbits im letzten Quantum ≠ 0 |
| `E_CHARSET` | kein Alphabetzeichen an Alphabetposition (inkl. `~`, Header, `=` außerhalb des Stromendes) |
| `E_PADDING` | Regel P verletzt |
| `E_MIXED_ALPHABET` | Regel A verletzt |
| `E_NON_URL_ALPHABET` | nur `decode_url_strict` |
| `E_FRAME_RULE` | Invariante F′ verletzt |
| `E_FRAME_SYNC` | erwarteter Frame-Header fehlt |

**Allokationsgrenzen.** Die Literallänge ist hart auf 4158 Bytes begrenzt, ein
Frame-Body auf 262143 chars. Daraus folgt: die Spezifikation braucht **kein
protokollseitiges Limit für einzelne Segmente oder Frames**, und es gibt keine
varint-Längen mit der zugehörigen Klasse von Einzelallokations-Bugs.

Daraus folgt **nicht**, dass gar kein Limit nötig wäre. Die Zahl der Segmente und
Frames ist unbegrenzt; ein Strom kann beliebig groß werden, beliebig viele Segmente
enthalten und beliebig große kumulative Ausgabe erzeugen. Implementierungen SOLLTEN
Gesamtgrößen- und Laufzeitlimits anbieten.

## 11. Kanonizität und Signaturen

**Der Encoder ist seit §9.0 eine Funktion**: (Eingabe, Preset, Profil) bestimmt
den Strom eindeutig. Jedes Preset ist entweder durch eine wahlfreie Regel
festgelegt (`dense`, `framed`, §9.2.1) oder durch eine Zielfunktion samt
Ordnung für Gleichstände (§11.1), und ein Encoder schreibt nur URL-Alphabet und
nie Padding (§5.1, §5.3).

Kanonisch ist das Format damit trotzdem nicht, aus zwei verbleibenden Gründen.
Erstens ist das **Preset und das Profil eine Wahl**: derselbe Input ergibt unter
`dense` und `canonical` verschiedene Ströme. Zweitens akzeptiert der **Dekoder
Formen, die kein Encoder schreibt** — das Classic-Alphabet (§5.2) und Padding
(§5.3). Ein Dritter kann denselben Strom also umschreiben, ohne die dekodierten
Bytes zu ändern. Regel A und Regel P halten diese Freiheit bei je Faktor 2.

> **Regel:** Signiere, hashe und vergleiche niemals die Ausgabe von `encode`.
> Signiere die **dekodierten Bytes**. `decode(encode(x)) == x` gilt immer.

**Base64 bleibt die ehrlichere Wahl,** wenn ein *fremdes* Protokoll die kodierte Form
signiert: es hat keine Parameter.

### 11.1 `encode_canonical` — vollständige Ordnung

#### Warum eine Byte-Klassifikation nicht reicht

Eine Ordnung über einem Bitvektor `isLiteral ∈ {0,1}^n` wäre nicht total, denn
`isLiteral` bestimmt die Ausgabe nicht eindeutig: Zwei angrenzende LiteralSegmente
sind nach §4 erlaubt und vom Dekoder unterscheidbar. Ein Literal-Lauf von `m` Bytes
kann deshalb als **ein** Segment oder als **mehrere** kodiert werden — bei
identischem `isLiteral`. Mit `h(m) = 2` für `m ≤ 62` und `h(m) = 4` für
`63 ≤ m ≤ 4158` gilt für einen Lauf von `m = m₁ + m₂`:

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

`encode_canonical(x, profile)` ist das Minimum von

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

**`canonical` ist kanonisch innerhalb eines festgelegten Profils**, nicht darüber
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
wird, und `B < L` entscheidet dann für das kürzere; TV13 ist das kleinste
Beispiel. v0.1 stand hier anders und war an dieser Stelle mit sich selbst im
Widerspruch.

**Zur Laufzeit.** Der Rückwärtslauf ist O(n) nach §9.2. Für den Vorwärtslauf ist
keine O(n)-Schranke bewiesen: die Deques liefern das Minimum, die Rekonstruktion
braucht das Argument des Minimums unter einer Tie-Break-Regel, und das ist eine
andere Anfrage. Eine Aufzählung der zulässigen Enden je Literal kostet
O(Fenster) je Literal.

**Verifikation.** Gegen erschöpfende Aufzählung **aller** gültigen
Segmentierungen bis `n ≤ 12`, über die drei Profile und über Alphabete, die
profil-legale und -widrige Bytes mischen: keine Abweichung zwischen DP und Brute
Force, bei mehr als fünfzig Eingaben mit echtem Längen-Gleichstand.

Die Fassung in v0.1 reichte bis `n ≤ 9` und fand deshalb nichts: die kleinste
Eingabe, auf der die alte *Berechnung* von der Ordnung abweicht, ist zehn Bytes
lang (TV13). Eine Suchraumgrenze ist eine Behauptung darüber, wo die Antwort
liegt, und gehört neben das Ergebnis.

Der Referenz-Encoder liegt der Spezifikation bei, und der Vektorsatz aus §16.8
ist veröffentlicht. Nachweis 3 in §16 ist damit **nicht** erbracht — er verlangt
zwei unabhängige Implementierungen, und es gibt eine.

#### Was `canonical` ausdrücklich nicht hat

* **kein `L_min`.** `canonical` minimiert die Länge und wählt deshalb Literale, wo
  immer sie kürzer sind — bei günstiger Ausrichtung bis hinab zu **`L = 7`**
  (9 statt 10 chars). `Ersparnis_worst(10) = 0` ist der **Worst Case**; im besten
  Fall spart dasselbe Literal 2 chars. Eine spätere Festlegung von `L_min` oder
  `B_min` (§9.5) DARF `canonical` deshalb nicht verändern — sonst änderten
  Messergebnisse rückwirkend bestehende Cache-Keys.
* **nicht die lineare Regel.** §9.2.1 ist für `dense` und `framed` normativ und
  für `canonical` unzulässig: sie ist nicht längenoptimal (§9.2.1, „Was sie
  kostet"), und Längenoptimalität ist die halbe Definition von `canonical`.
* **kein Framing, kein Classic-Alphabet, kein Padding.**

**Verwendung:** Cache-Keys, Dedup-Keys, Content-Addressing, Testvektoren.
**Nicht** für Signaturen — dort gilt §11.

## 12. Dichte

Die beiden mittleren Zeilen sind exakt, die beiden unteren gemessen auf
erzeugten Eingaben der angegebenen Form (`cargo run --release --example
density`, 1 MiB je Zeile, Profil U). Erzeugte Eingaben sind kein Korpus: die
Zahl hängt daran, wie gemischt wird, und deshalb verlangt §16.5 dafür
binary2textbench.

| Eingabe | Base64 | **Base65t** | Z85 | basE91 |
|---------|--------|-------------|-----|--------|
| Rein binär (Profil U, T) | 1,333 | **1,333** *(exakt)* | 1,250 | 1,231 |
| Rein profil-legaler Text | 1,333 | **≤ 1,00096** *(exakt, langer Literalbereich)* | 1,250 | 1,231 |
| 70 % Text / 30 % binär | 1,333 | *1,113 (gemessen)* | 1,250 | 1,231 |
| 30 % Text / 70 % binär | 1,333 | *1,244 (gemessen)* | 1,250 | 1,231 |

Zur ersten Zeile: sie gilt für Profil U und T. Unter Profil B ist jedes Byte
literalfähig, der Encoder schreibt ein einziges Literalsegment, und die Dichte
ist auch für Binärdaten 1,00096. Profil B ist kein Textencoding (§7), und die
Tabelle beschreibt es nicht.

Zur zweiten Zeile: `4162/4158 = 1,00096` gilt für einen maximalen Literalblock und
ist eine **exakte Schranke**, kein Grenzwert. Da Literale bei 4158 Bytes gedeckelt
sind, nähert sich die Dichte langer Eingaben dieser Konstanten an, nicht der 1.

**URL-Query — gilt ausschließlich für Profil U:**

| Container | Base64url | **Base65t/U** | Base65t/T | Base85N | Base91z |
|-----------|-----------|---------------|-----------|---------|---------|
| URL-Query | 1,333 | **≤ 1,333, bei Text bis 1,001** | prozent-encoding-pflichtig | *(1,463 über Korpus)* | nicht geeignet |

Der URL-Vorteil ist ein Vorteil **von Profil U**, nicht des Formats an sich.

## 13. Performance

Base64 hat null datenabhängige Branches, Base65t einen pro Segmentwechsel. Bei fein
durchmischten Daten ist Base65t deshalb langsamer.

Gemessen über den Korpus von `binary2textbench` (68 Proben, 6,5 MB, Base64 =
100 %):

| | Kodieren | Dekodieren | Größe |
|---|---|---|---|
| ohne Kompressor | 114 % | 107 % | 132,0 % (Base64: 133,3 %) |
| mit zstd −5 davor | 103 % | 103 % | 56,1 % (Base64: 56,6 %) |
| mit zstd 1 davor | 101 % | 100 % | 40,6 % (Base64: 40,6 %) |

Der Korpus ist nach Bytes gewichtet, also von Dateien im Megabyte-Bereich
bestimmt. Auf kurzen Werten — dem, wofür §0.1 das Format vorsieht — sieht es
anders herum aus; die Tabelle dafür steht weiter unten.

Alles einthreadig gemessen, wie der Bench jeden Codec misst. §9.2.1.1 sagt,
warum `dense` sich aufteilen lässt, ohne dass sich ein Byte ändert; auf vier
Kernen bringt das auf Prosa — dem schlechtesten Fall oben — 305 auf 534 MiB/s.

Die dritte Zeile ist der Normalfall in einem Protokoll, das komprimiert: dort
ist die Eingabe hochentropisch, `dense` schreibt nach §9.4 **dieselben Bytes**
wie Base64URL, und was bleibt, ist das Suchen nach Literalen, die es nicht gibt.

**Beide Seiten sind skalar.** Die Base64-Referenz im Bench ist derselbe Bau wie
unsere: dieselbe Tabelle, dieselbe Schleifenform, derselbe Compiler mit
denselben Schaltern. Der Vergleich misst also das Format und keinen
Handicap-Unterschied — was auch heißt, dass die Zahlen nichts darüber sagen,
wie sich zwei *vektorisierte* Implementierungen zueinander verhielten. §13.1
beschreibt, wie die vektorisierte Seite für Base65t aussieht.

**Die Kosten hängen an der Segmentwechselrate, und das ist messbar.** §9.5
sagt, dass die Rate exakt und datenabhängig ist; hier ist, was sie vorhersagt.
Je Datei: Eingabebytes je Segment, Größe gegen Base64, Zeit gegen Base64
(unter 100 % ist schneller):

| Datei | Bytes je Segment | Größe | Kodieren | Dekodieren |
|---|--:|--:|--:|--:|
| `random.bin` | 262 144 | 100,0 % | 121 % | 84 % |
| `sql-wasm.wasm` | 936 | 99,9 % | 106 % | 85 % |
| `DejaVuSans.ttf` | 782 | 99,9 % | 110 % | 86 % |
| `countries.json` | 190 | 99,6 % | 140 % | 99 % |
| `commonmark-spec.txt` | 144 | 99,5 % | 193 % | 98 % |
| `lodash.js` | 67 | 98,7 % | 196 % | 124 % |
| `requests-2.32.3.tar` | 40 | 96,9 % | 180 % | 131 % |
| `bootstrap.css` | 19 | 93,2 % | 189 % | 161 % |

**Beide Richtungen folgen der Rate**, und beide liegen bis etwa 150 Bytes je
Segment nahe an der Base64-Parität. Darunter steigen sie mit der Zahl der
Wechsel — was §9.5 vorhersagt und wofür es dort auch den Hebel gibt (`B_min`,
als **eigenes** Preset).

Das war einmal anders, und der Weg dahin ist der eigentliche Befund. Der
Encoder las die Eingabe byteweise und fragte je Byte, ob das Profil es zulässt.
Auf Prosa in Profil U — Leerzeichen alle fünf Zeichen, also lauter fünf Byte
lange Läufe, von denen keiner die Schwelle aus §9.1 erreicht — ist diese Frage
für den Sprungvorhersager nicht zu erraten, und es fällt eine Fehlvorhersage je
Lauf an. `dickens` kostete so 205 % statt der 113 % oben, bei nahezu derselben
Wechselrate wie `countries.json` mit 136 %. Aufgelöst hat das die Bitmaske aus
§9.2.1.2.

### Auf kurzen Werten ist Base65t schneller als Base64

Die Tabelle oben misst Dateien von mehreren Megabyte. Dafür ist das Format
nicht gemacht: §0.1 nennt URL-Query, Cookie-Wert, HTTP-Header und Cache-Key,
und keiner davon ist acht Megabyte groß. Bei acht Megabyte sind beide Codecs
durch die Speicherbandbreite begrenzt und nicht durch das, was sie rechnen — der
Quotient misst dann den Scan. Bei vierundsechzig Byte misst er das Format.

Dieselben 55 kurzen Proben, die `binary2textbench` als `short/` führt, Profil U,
Base64 = 100 % (bestes von fünf Läufen; die Dekodierspalte streut auf so kurzen
Werten um mehrere Punkte zwischen Läufen, die Kodierspalte kaum):

| Probe | Bytes | Größe | Kodieren | Dekodieren |
|---|--:|--:|--:|--:|
| SHA-256-Digest, hex | 64 | 77 % | **58 %** | **82 %** |
| SHA-512-Digest, hex | 128 | 77 % | **87 %** | **69 %** |
| AES-256-Schlüssel, hex | 64 | 77 % | **57 %** | **80 %** |
| JWT, drei Segmente | 155 | 76 % | **63 %** | **71 %** |
| Session-ID, 40 alnum | 40 | 75 % | **69 %** | **89 %** |
| UUID v4 | 36 | 79 % | **80 %** | **89 %** |
| ULID, Crockford | 26 | 78 % | **83 %** | **90 %** |
| zufällige 64 Bytes | 64 | 98 % | 102 % | 108 % |
| IPv6-Adresse | 28 | 95 % | 111 % | 125 % |
| SQL-Statement | 118 | 98 % | 108 % | 109 % |
| Logzeile | 93 | 95 % | 110 % | 137 % |
| **alle 55 Proben, als Zeit** | | | **86 %** | **~100 %** |

**Der Durchsatzvorteil *ist* der Dichtevorteil**, und zwar fast eins zu eins:
wo die Ausgabe 75–79 % der Base64-Länge hat, kostet das Kodieren 57–83 % der
Zeit.
Das ist kein Zufall, sondern die Arbeitsbilanz. Base64 liest ein Byte, schlägt
vier Zeichen nach und schreibt vier — je drei Bytes. Ein Literal liest ein Byte,
prüft es gegen die Profilmenge und schreibt **ein** Zeichen; das Schreiben ist
ein `memcpy`. Wer weniger schreibt, schreibt schneller.

Die Umkehrung gilt genauso und steht in denselben Zeilen: wo die Ausgabe
gleich groß ist wie Base64 (95–98 %), ist Base65t langsamer, und zwar genau um
das, was das Suchen kostet. Ein Literal, das nicht zustande kommt, ist Arbeit
ohne Gegenwert.

> **Als Faustregel, aus der Bilanz und nicht aus einer Messung:** Base65t ist
> auf einem Wert ungefähr so viel schneller, wie es kürzer ist — und wo es
> nicht kürzer ist, ist es um den Scan langsamer.

**Kodieren auf großen Dateien folgt der Wechselrate nicht** — und das ist der
interessantere Befund.
`commonmark-spec.txt` und `countries.json` haben fast dieselbe Wechselrate und
unterscheiden sich beim Kodieren um den Faktor 1,4. Was den Encoder kostet,
sind nicht die Segmente, die entstehen, sondern die Läufe, die er **prüft und
verwirft**: englische Prosa in Profil U hat alle fünf Zeichen ein Leerzeichen,
also lauter profil-legale Läufe von fünf Bytes, von denen keiner die Schwelle
aus §9.1 erreicht. Der Encoder liest sie alle und schreibt am Ende reines
Base64 (99,5 % Größe). Das ist die Arbeit, die der Sprung aus §9.2.1 gerade
nicht wegnehmen kann: er springt nur über Bytes, die die Profilmenge verlassen,
und hier verlässt sie fast keines.

Wer das nicht bezahlen will, hat zwei Wege, und beide stehen schon im Dokument:
Profil T (dort ist das Leerzeichen enthalten, die Läufe sind lang, und derselbe
Text wird ein einziges Literal) oder ein `B_min > 1` als **eigenes** Preset
(§9.5).

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

### 13.1.1 Was Vektorisierung wirklich bringt, gemessen

Die Referenzimplementierung kann das Base64-Schreiben **und das
Base64-Lesen** hinter dem Feature `simd` an einen vektorisierten Kern abgeben.
Die Ausgabe ändert sich dabei nicht um ein Byte — Base64 ist Base64 — also ist
es ein Geschwindigkeitsschalter wie die Arbeiterzahl (§9.2.1.1), kein
Formatthema.

**Beim Dekoder ist das nicht selbstverständlich**, und es lohnt sich, den Grund
zu benennen. Eine Base64-Bibliothek legt sich je Aufruf auf **ein** Alphabet
fest und meldet **einen** Fehler; §5.2 verlangt, dass beide Varianten gelesen
werden, §5.4 verlangt zu wissen, welche es war, und §10.4 verlangt zwölf
unterscheidbare Bedingungen. Was das rettet, ist eine Beobachtung: Regel A
braucht nur die Frage *„steht in diesem Lauf ein `+`, `/`, `-` oder `_`"*, und
das ist eine **Suche und keine Dekodierung**. Als eigener Durchgang gefragt
kostet sie ein Siebtel dessen, was das Dekodieren kostet — und ihre Antwort
wählt anschließend das Alphabet für den Bibliotheksaufruf. Bleibt der Fehlercode:
schlägt der vektorisierte Aufruf fehl, läuft die skalare Schleife noch einmal
über denselben Lauf und benennt die Bedingung. Das ist per Definition der
langsame Pfad — er läuft einmal, auf einem Strom, der ohnehin abgelehnt wird.

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

**Und damit die eigentliche Antwort auf „schlagen wir Base64":**

| 8 MB | `dickens` | `mozilla` | `countries.json` |
|---|--:|--:|--:|
| Kodieren, skalar gegen skalare Base64 | 113 % | 111 % | 112 % |
| Kodieren, **mit `simd`** gegen skalare Base64 | **80 %** | **79 %** | **74 %** |
| Dekodieren, skalar gegen skalare Base64 | 103 % | 104 % | 103 % |
| Dekodieren, **mit `simd`** gegen skalare Base64 | **77 %** | **72 %** | **70 %** |
| Kodieren, mit `simd` gegen **vektorisierte** Base64 | 388 % | 354 % | 355 % |

Die dritte Zeile ist die ehrliche. Gegen eine vektorisierte Base64 verliert
Base65t auf großen, hochentropen Daten deutlich, und der Grund ist strukturell
und nicht behebbar: **Base64 schaut nicht, es schreibt nur.** Base65t muss die
Eingabe erst lesen, um zu wissen, ob ein Literal darin steckt, und dieser Blick
ist auf Daten, in denen keines steckt, reine Zusatzarbeit. Selbst mit einer
vektorisierten Profilprüfung bliebe es bei zwei Durchgängen gegen einen.

Wo Literale zustande kommen, dreht sich das um, und zwar aus demselben Grund:
dort wird weniger geschrieben (§13, „Auf kurzen Werten"). Die Regel bleibt, was
sie war — Base65t ist so viel schneller, wie es kürzer ist, und wo es nicht
kürzer ist, ist es um den Blick langsamer.

**Nicht untersucht, und warum nicht:**

* **GPU.** Die Daten müssten über PCIe und zurück. Für die Werte aus §0.1 —
  URL-Query, Cookie, Cache-Key — kostet allein die Latenz mehr als das
  Kodieren, und für Massendaten ist die Parallelisierung aus §9.2.1.1 auf
  gewöhnlichen Kernen näher und exakt.
* **DMA.** Ein Gerät, das Speicher umkopiert, hilft beim Kopieren; das hier ist
  kein Kopieren. Die Maschine im Bench schafft 7 GB/s `memcpy` und dieser
  Encoder 0,8 — der Engpass ist die Rechnung, nicht der Transport.

### 13.2 Das Durchsatz-Kriterium

v0.1 sah hier vier Schwellwerte *X*, *Y*, *Z* vor, „vor der Messung
festzulegen". Sie waren erfunden, und sie sind gestrichen. An ihre Stelle tritt
eine Regel, die aus den Zielen folgt statt aus einer gewünschten Zahl:

> **Durchsatz ist ein Ziel, Größe ist eine Zusicherung.** Eine Änderung an einem
> Preset DARF die Zusicherung aus §9.4 nicht antasten. Innerhalb dieser
> Schranke SOLL sie den Durchsatz verbessern; eine Änderung, die Durchsatz
> gegen Dichte tauscht, ist ein **neues Preset** (§9.5), keine neue Fassung
> eines bestehenden.

Woran das gemessen wird:

* **Auf hochentropen Daten schreibt `dense` dieselben Bytes wie Base64URL.**
  Dort ist Parität keine Zusage, sondern Identität — es ist derselbe Strom. Was
  bleibt, ist der Scan, der nach Literalen sucht, die es nicht gibt; §9.2.1
  macht daraus eine Prüfung je elf Bytes statt je Byte.
* **Für alles andere berichtet die Bench**, mit den Zahlen in §13 und einem
  Korpus, der veröffentlicht ist. Die Segmentwechselrate ist exakt und
  deterministisch (§9.5) und erklärt die Zahlen, ohne sie zu ersetzen.
* **Skalar gegen vektorisiert ist kein Ergebnis.** Ein Vergleich zählt, wenn
  beide Seiten denselben Grad an Handarbeit gesehen haben. Im Bench ist das so,
  und deshalb gilt er; §13.1 sagt, wie die vektorisierte Seite aussähe.
* **Die eigene Schleife ist zuerst verdächtig, nicht das Format.** Der Abstand
  beim Dekodieren war einmal 158 % und lag zu keinem Teil am Format: er lag an
  zwei Schleifen der Referenzimplementierung, die byteweise arbeiteten, wo das
  Format es nicht verlangt. Wer eine Zahl aus §13 nicht erreicht, prüft das
  zuerst.

Ein Encoder oder Dekoder ist nicht deshalb unkonform, weil er langsamer ist als
Base64. Er ist es, wenn er die falschen Bytes schreibt.

## 14. Sicherheit

* **Der Dekoder parst angreiferkontrollierte Längen. Base64 tut das nicht.** Ein
  Nachteil gegenüber Base64. Zurückgezahlt durch die harte 4158-Byte-Grenze, die
  Prüfliste in §10.4, Fuzzing und speichersichere Referenzimplementierungen. Eine
  speichersichere Sprache ist die *Bezahlung* dieser Angriffsfläche, kein Argument
  gegen Base64.
* **Literale lecken Struktur** — Klartextanteil und alle Lauflängen sind sichtbar.
  Dafür ist das Preset `opaque` da; seine Ausgabe ist identisch mit Base64URL.
* **Drei Auto-Erkennungen sind drei Parser-Differential-Flächen:** Alphabet (§5.2),
  Padding (§5.3), Framing (§5.6). Liest eine Komponente permissiv und eine andere
  strikt, entstehen zwei Wahrheiten über denselben Strom. Gegenmaßnahmen: Regel A,
  Regel P, `alphabet_seen` / `padding_seen` / `framing_seen`, und die strikten
  Varianten aus §5.5. Differential-Fuzzing ist Pflicht, nicht Kür.
* **Kein Padding-Orakel** — Padding wird nur validiert, nie erzeugt.
* **Malleability** ausgeschlossen auf Segmentebene, reduziert auf Alphabet- und
  Padding-Ebene, **nicht** auf Segmentierungsebene (§11). Dafür ist `canonical` da.
* Dekodierte Ausgabe ist **untrusted binary**, nicht Text.

## 15. Testvektoren

### TV1–TV4 — Grundfälle (Profil U)

```
TV1  "alice.jones"                      -> ~Lalice.jones      (13 vs. 15 Base64)
TV2  DE AD BE EF "session-eu-central"   -> 3q2-7w~Ssession-eu-central   (26 vs. 30)
TV3  "sub~alice~jones"                  -> ~Psub~alice~jones  (17 vs. 20)
TV4  Literal von 100 Bytes              -> Header ~_Al   (Classic: ~/Al)
     L1 = 63, V = 100 − 63 = 37 = 000000 100101 -> 'A'(0), 'l'(37)
```

Base64URL-Vergleichswerte: `YWxpY2Uuam9uZXM`,
`3q2-73Nlc3Npb24tZXUtY2VudHJhbA`, `c3VifmFsaWNlfmpvbmVz`.

TV2 ist eine Byte-Zusicherung und keine Längenzusicherung: drei Segmentierungen
sind 26 Zeichen lang, weil das Aufnehmen von ein oder zwei Textbytes in das
Base64-Segment nichts kostet (`ceil(4k/3) + (22 − k) + 2 = 26` für k = 4, 5, 6).
Die Zeile oben ist `dense`, also die lineare Regel aus §9.2.1: sie nimmt das
Literal, sobald es beginnt, und wählt damit k = 4. `canonical` wählt k = 6, weil
die Ordnung aus §11.1 an Index 4 `B < S` sieht:

```
dense     : 3q2-7w~Ssession-eu-central   (26 chars)
canonical : 3q2-73Nl~Qssion-eu-central   (26 chars)
```

Gleich lang, verschiedene Bytes — der Fall, für den §9.0 verlangt, dass jedes
Preset eine Funktion ist. v0.1 führte hier die `dense`-Zeile, ohne eine Regel zu
nennen, die sie auszeichnet; v0.2 nennt sie.

### TV5 — F1/F2-Konflikt

**Diese Vektoren sind Encoder-Segmentierungen, keine vollständigen Ströme.**

**5a — Segmentierung (Frame-Body), Profil U:**

```
Input            : "hello~Alice"     (11 Bytes)
Literal-Versuch  : ~Lhello~Alice     (13 chars) -> F1-VERSTOSS, ungültig
Body (dense)     : aGVsbG9-QWxpY2U   (15 chars) -> reines Base64
Body (canonical) : aGVsbG9-QWxpY2U   (15 chars) -> ebenso
```

Der dichte Encoder wählt **kein** Literal — der erzwungene Moduswechsel (§8.2) ist
teurer als durchgehendes Base64. Regressionstest für die DP-Kostenfunktion.

v0.1 gab hier für das inzwischen gestrichene `legible` `~Fhellofg~FAlice` an,
16 Zeichen. Das ist ein gültiger Strom und dekodiert korrekt, aber kein Encoder
schreibt ihn: jedes Preset minimiert nach §9.0 zuerst die Länge, und 15 ist
kürzer als 16.

**5b — dieselben Bodies als vollständige Framed-Ströme:**

```
encodiert: ~A + AAP + aGVsbG9-QWxpY2U  =  ~AAAPaGVsbG9-QWxpY2U    (20 chars)
           'AAP' = 18-Bit-Länge 15
nur lesbar: ~A + AAQ + ~Fhellofg~FAlice = ~AAAQ~Fhellofg~FAlice   (21 chars)
           'AAQ' = 18-Bit-Länge 16
```

Die erste Zeile ist, was `framed` schreibt. Die zweite schreibt kein Encoder
(siehe 5a), ist aber ein gültiger Strom und dekodiert zu denselben Bytes — ein
Dekodertest, kein Encodervektor. Beide enthalten `~A` ausschließlich an Index
0–1, F′ ist gewahrt. Nur **5b** darf an `decode()` übergeben werden; **5a** ist
an `decode_plain()` zu übergeben oder als Body innerhalb eines Frames zu prüfen.

### TV6 — Abwärtskompatibilität

```
Bytes "<<???>>" = 3C 3C 3F 3F 3F 3E 3E

PDw_Pz8-Pg      -> "<<???>>"      alphabet_seen = url,     padding_seen = false
PDw/Pz8+Pg      -> "<<???>>"      alphabet_seen = classic, padding_seen = false
YWxpY2Uuam9uZXM -> "alice.jones"  alphabet_seen = none
YWxpY2U=        -> "alice"        alphabet_seen = none,    padding_seen = true
```

### TV7 — Alphabet-Konsistenz

```
PDw_Pz8-Pg  -> gültig, url         PDw_Pz8+Pg -> E_MIXED_ALPHABET
PDw/Pz8+Pg  -> gültig, classic     PDw/Pz8-Pg -> E_MIXED_ALPHABET
PDw/Pz8+Pg  -> E_NON_URL_ALPHABET  in decode_url_strict
```

**Negativtest — Payload zählt nicht:** In Profil T ist `~Ka+b/c-d_e~fg …` gültig; die
Zeichen `+ / - _` stehen in einer Literal-Payload, nicht an Alphabetpositionen. Ein
Dekoder, der Regel A auf den Gesamtstrom anwendet, weist das fälschlich ab.

### TV8 — Header-Zeichenvalidierung

```
~~abc  -> E_CHARSET          ~_A~  -> E_CHARSET
~=ab   -> E_CHARSET          ~     -> E_TRAILING_TILDE
```

### TV9 — F′ vs. F2, mit vollständigen Strömen

F′ ist eine Aussage über den **kodierten Strom**, nicht über die semantische
Segmentart — die Vektoren geben ihn deshalb vollständig an.

**9a — F2 verletzt, F′ verletzt → `E_FRAME_RULE`**

```
Frame-Body : ~Cx~AA
             ~C   Literal-Header, L = 2
             x~   Payload "x~"          <- endet auf '~' (F2 verletzt)
             AA   Base64-Segment, n = 2 -> 1 Byte 0x00
Vollständiger Frame: ~AAAG~Cx~AA        ('AAG' = 18-Bit-Länge 6)
Index         :      0123456789A
Spurious "~A" bei Index 8–9             -> E_FRAME_RULE
```

**9b — F2 verletzt, F′ gewahrt → gültig**

```
Frame-Body : ~Cx~~Cyz
             ~C   Literal-Header, L = 2
             x~   Payload "x~"          <- endet auf '~' (F2 verletzt)
             ~C   Literal-Header, L = 2
             yz   Payload "yz"
Vollständiger Frame: ~AAAI~Cx~~Cyz      ('AAI' = 18-Bit-Länge 8)
Kein "~A" außer dem Frame-Header        -> gültig, dekodiert zu "x~yz"
```

9b MUSS akzeptiert werden. Ein Dekoder, der F2 statt F′ prüft, weist es
fälschlicherweise ab — das ist der Fehler, den §8.2 ausschließt, und dies ist sein
Regressionstest. Beide Vektoren sind **Dekoder**-Tests; die `dense`-Schwellwerte aus
§9.1 gelten für sie nicht.

### TV10 — Padding

```
YWxpY2U=      -> "alice"    gültig  (n=7, 7 mod 4 = 3, k=1)
YWxpY2Uu      -> "alice."   gültig  (n=8, k=0)
YWxp==        -> E_PADDING          (n=4, k=2 verlangt n mod 4 = 2)
YWxpY2U==     -> E_PADDING          (n=7 verlangt k=1)
YWxpY2U=~Lfoo -> E_CHARSET          ('=' nicht am Stromende)
```

**Negativtest — Profil T, `=` als Literal-Byte:**

```
~Da=b=   ('D' = 3, Payload "a=b", danach Base64-Segment "=" mit n=0) -> E_PADDING
~Ea=b=   ('E' = 4, Payload "a=b=", danach Stromende)  -> "a=b=", padding_seen = false
```

Beide Ströme enden auf `=`. Der Unterschied liegt allein in der Literal-Länge, also
darin, ob der Scanner die Position überhaupt erreicht. Genau deshalb ist
Vorab-Strippen falsch.

### TV11 — Framing-Erkennung (Regel F §5.6)

```
""              -> gültig, 0 Bytes, framing_seen = plain (Konvention)
"YWxpY2U"       -> framing_seen = plain
"~Lalice.jones" -> framing_seen = plain   ('L' = 11 ≠ 0)
"~AAAI~Cx~~Cyz" -> framing_seen = framed
"~Aabc"         -> decode()       : framed, dann E_TRUNCATED
                   decode_plain() : E_RESERVED_LEN
```

`E_FRAME_SYNC` ist an dieser Stelle nicht erreichbar: §10.3 prüft Marker, dann
Länge, und `abc` ist eine wohlgeformte 18-Bit-Länge von 108252, die ein fünf
Zeichen langer Strom nicht erfüllen kann. v0.1 ließ hier beide Codes zu.

Die letzte Zeile ist der Kern von Regel F: **derselbe Strom**, zwei Einstiegspunkte,
zwei Fehler. `decode()` erkennt `framed` und meldet den Framed-Fehler;
`decode_plain()` erzwingt Plain und meldet `E_RESERVED_LEN`. Beides ist korrekt und
darf nicht vermischt werden.

### TV12 — Fehlerfälle

| Eingabe | Erwartet |
|---------|----------|
| `abcde` | `E_ALIGN` |
| `~Aabc` (via `decode_plain`) | `E_RESERVED_LEN` |
| `~L` + nur 3 Bytes | `E_TRUNCATED` |
| `~Cab~` | `E_TRAILING_TILDE` |
| `YWxp==` | `E_PADDING` |
| `~Ca b` (Profil U) | `E_PROFILE` |
| Base64 mit gesetzten Restbits | `E_NONZERO_TAIL` (erwartete Abweichung von permissiven Base64-Bibliotheken, §1.1) |

### TV13 — die Tie-Break-Regel (§9.0, §11.1)

Die kleinste Eingabe, auf der die Regel überhaupt etwas entscheidet: neun
profil-legale Bytes, dann eines, das die Profilmenge nicht enthält.

```
Input "aaaaaaaaa "  (10 Bytes, Profil U)

canonical : ~HaaaaaaaYWEg   (13 chars)   c = SLLLLLLBBB
verworfen : ~JaaaaaaaaaIA   (13 chars)   c = SLLLLLLLLB
```

Beide sind längenoptimal. An Index 7 steht `B` gegen `L`, und `B < L`
entscheidet für das kürzere Literal — ein Literal früh zu beenden richtet den
Base64-Lauf so aus, dass die restlichen drei Bytes zusammen zwei Zeichen
sparen. Die zweite Zeile ist, was die *Berechnung* in v0.1 lieferte; sie ist
hier der Negativvektor.

`dense` schreibt hier `YWFhYWFhYWFhIA` (14 chars, reines Base64): der Lauf ist
neun Bytes lang und erreicht die Schwelle aus §9.1 nicht. Das ist genau der
Unterschied, den §9.3 beziffert — und die Grenze aus §9.4 hält auch hier,
`ceil(40/3) = 14`.

### TV14 — zurückgezogen

TV14 stellte `legible` gegen `dense` auf derselben Eingabe. `legible` ist
gestrichen (§9.3), und die Nummer wird **nicht neu vergeben**: eine Verweisung
auf TV14 aus einem älteren Dokument soll hier landen und nicht auf etwas
anderem, das ihren Namen trägt.

### TV15 — Padding reicht nicht in einen Frame (§5.3)

```
YWxpY2U=       -> "alice", padding_seen = true      (Plain, Stromende)
~AAAIYWxpY2U=  -> E_CHARSET                         ('AAI' = Länge 8)
~AAAHYWxpY2U   -> "alice"                           ('AAH' = Länge 7)
```

Derselbe Base64-Text, einmal als Strom und einmal als Frame-Body. Im Frame ist
`=` kein Padding, sondern ein Zeichen ohne Wert an einer Alphabetposition. In
v0.1 war das nicht entschieden: §8.1 nennt einen Body einen Plain-Mode-Stream,
§5.3 spricht vom Strom, und beide Lesarten waren vertretbar.

## 16. Konformitätsnachweise

Eine Implementierung gilt als konform, wenn sie die vier folgenden Eigenschaften
belegt:

1. **`decode(encode(x)) == x`** für alle Profile, alle Presets, über Fuzzing-Korpus.
2. **`decode(base64(x)) == x`** und **`decode(base64url(x)) == x`** für alle
   kanonischen Eingaben, gepaddet und ungepaddet — per Differential-Fuzzing gegen die
   Standard-Base64-Bibliothek der jeweiligen Sprache. Erwartete Abweichungen
   (`E_NONZERO_TAIL`, §1.1) gehören als solche in den Korpus.
3. **`encode_canonical(x)` byte-identisch über zwei unabhängige Implementierungen**,
   über den gesamten Vektorsatz. Ohne diesen Test ist §11.1 eine Behauptung.
   **Erbracht, mit einer benannten Lücke.** Zwei Implementierungen liegen bei:
   `rust/` und `conformance/reference.py`, die zweite aus diesem Dokument
   geschrieben,
   mit einem quadratischen DP statt der Schiebefenster aus §9.2 und ohne eine
   Zeile gemeinsamen Code. Sie stimmen über alle 449 Vektoren, alle fünf
   Presets und alle drei Profile byteweise überein — 870 Paare — und über
   fünfzehn Fehlerfälle dazu, was ebenso zählt: wer sich über gültige Ströme
   einig ist und über ungültige nicht, ist sich über das Format nicht einig.
   Über den Vektorsatz hinaus kodieren beide dieselbe Viertelmegabyte-Eingabe
   und schreiben Zeichen für Zeichen denselben Strom
   (`conformance/test_large.py`) — was erst möglich ist, seit `dense` nach
   §9.2.1 linear ist; über den quadratischen DP war eine Eingabe dieser Größe
   in Python nicht zu kodieren.
   Die Lücke: derselbe Autor. Eine dritte Implementierung von jemand anderem
   prüft sich gegen `docs/vectors.json`, ohne eine der beiden zu lesen.
4. **Keine Fehlsynchronisation auf `~A` im Framed Mode**, auch bei adversarialen
   Literalbytes — gezielt gegen F1/F2/F′ gefuzzt, mit TV9a/9b als Startpunkt.

Ergänzende Arbeiten, nicht normativ:

5. Messen (§12, §13): Korpusdichte und Durchsatz über binary2textbench —
   **erbracht**, die Zahlen stehen in §13. Base65t ist dort als siebter Codec
   eingehängt und wird bei jeder Änderung mitgemessen. Das Kriterium steht in
   §13.2; ein Ergebnis, das Dichte gegen Durchsatz tauschen will, begründet nach
   §9.5 ein neues Preset und keine neue Fassung eines bestehenden.
6. Container-Test mit echten Parsern — **erledigt für Pythons Parser**,
   `conformance/test_containers.py`: URL-Query gegen `urllib.parse`, Cookie gegen
   `http.cookies`, JSON gegen `json`, dazu Dateiname und Logzeile. Profil U
   geht durch alle unverändert; Profil T braucht in einer URL Prozent-Encoding
   und enthält das Leerzeichen — beides Negativkontrollen, und die zweite hat
   den Zusatz in §7 hervorgebracht. Ein Parser-Satz, nicht alle: Browser,
   Proxies und Frameworks bleiben offen.
7. API-Form je Zielsprache: `encode` / `decode` analog zum dortigen `base64`;
   zusätzlich `decode_url_strict`, `decode_plain`, `decode_framed`,
   `encode_canonical`, `encode_opaque`, `encode_framed`.
   Rust liegt bei; `python/` ist ein PyO3-Binding darüber und exportiert
   dieselbe Menge, damit ein Python-Aufrufer byteweise dasselbe bekommt wie ein
   Rust-Aufrufer. Ein Binding ist ausdrücklich **keine** zweite Implementierung
   im Sinne von Nachweis 3 — es kann der ersten gar nicht widersprechen.
8. Vektorsatz auf ≥ 200 ausbauen — **erledigt**: `docs/vectors.json` führt 449
   Vektoren über alle fünf Presets und alle drei Profile, jeder als Eingabe und
   erwarteter Strom in Hex. Der Fuzzing-Korpus für alle zwölf Fehlercodes
   liegt in der Testsuite der Referenzimplementierung.

## 17. Erweiterungskandidaten (nicht Teil von v0.1)

1. **Profil-Aushandlung.** Aus dem Strom prinzipiell nicht ableitbar (§7.2); ein
   1-char-Präfix wäre selbstbeschreibend, kostet aber ein Zeichen.
2. **Frame-Prüfsumme.** CRC32C pro Frame (6 chars) für Storage sinnvoll, für URLs
   Verschwendung.
3. **Case-insensitive Profil.** Bräuchte einen Base32-Kern — im Grunde ein eigenes
   Format.
