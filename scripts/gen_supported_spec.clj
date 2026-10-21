#!/usr/bin/env bb
;; Regenerates docs/SUPPORTED_SPEC.md — the Wasm instruction support matrix —
;; straight from the binary encoder, so the doc cannot drift from what
;; encode_instr actually emits. Run from anywhere:
;;   bb scripts/gen_supported_spec.clj
;;
;; Sources of truth (parsed, never hardcoded):
;;   crates/gleam-wasm/src/ir/types.rs          -> ordered Instr variant list
;;   crates/gleam-wasm/src/emit/wasm/encoder.rs -> variant -> opcode bytes
;; Output is deterministic: no timestamps, enum order preserved.

(require '[clojure.string :as str]
         '[babashka.fs :as fs])

(def root
  (-> *file* fs/expand-home fs/canonicalize fs/path
      (.getParent) (.getParent) str))

(defn repo-path [rel] (str (fs/path root rel)))

(def types-path (repo-path "crates/gleam-wasm/src/ir/types.rs"))
(def encoder-path (repo-path "crates/gleam-wasm/src/emit/wasm/encoder.rs"))
(def out-path (repo-path "docs/SUPPORTED_SPEC.md"))

;; --- 1. Ordered Instr variants from the enum in types.rs --------------------

(defn enum-variants []
  (let [body (-> (slurp types-path)
                 (str/split #"pub enum Instr \{")
                 second
                 (str/split #"\n\}\n")
                 first)]
    (->> (str/split-lines body)
         ;; variant lines are 4-space indented and start uppercase; struct
         ;; fields (8 spaces, lowercase), comments and `},` never match.
         (mapcat #(re-seq #"^\s{4}([A-Z][A-Za-z0-9]*)" %))
         (map second)
         vec)))

;; --- 2. Opcode arms from encode_instr in encoder.rs -------------------------

(defn- push-byte [line]
  (when-let [b (second (re-find #"buf\.push\((0x[0-9A-Fa-f]{2})\)" line))]
    (Integer/parseInt (subs b 2) 16)))

(defn- u32-lit [line]
  (when-let [n (second (re-find #"encode_u32\(buf,\s*(\d+)\)" line))]
    (Integer/parseInt n)))

(defn- transparent?
  "Lines inside a multi-line arm header (fields, `} => {`) or comments that
  must not stop opcode collection."
  [line]
  (or (str/blank? line)
      (re-find #"^\s*//" line)
      (re-find #"^\s*\w+: [\w:<>()]+,\s*$" line)
      (re-find #"^\s*\w+,?\s*$" line)
      (re-find #"^\s*\} => \{\s*$" line)))

(defn- arm-opcode [{:keys [pushes const]}]
  (when (seq pushes)
    (let [hexes (map #(format "0x%02X" %) pushes)]
      (if (and (#{0xFB 0xFC} (first pushes)) const)
        (str (str/join " " hexes) " " (format "0x%02X" const))
        (str/join " " hexes)))))

(defn- flush-arm [acc]
  (if-let [c (:current acc)]
    (-> acc
        (update :table conj [c (arm-opcode (:state acc))])
        (assoc :current nil :state nil))
    acc))

(defn- step [acc line]
  (cond
    ;; A new arm starts; a one-line arm carries its push on the same line.
    (re-find #"Instr::(\w+)" line)
    (let [v (second (re-find #"Instr::(\w+)" line))
          b (push-byte line)]
      (-> (flush-arm acc)
          (assoc :current v
                 :state {:pushes (if b [b] [])
                         :const (u32-lit line)
                         :stopped false})))

    ;; `_ =>` fallback: flush the last arm, nothing after is encoded.
    (re-find #"^\s*_ =>" line)
    (flush-arm acc)

    (or (nil? (:current acc)) (get-in acc [:state :stopped]))
    acc

    (transparent? line)
    acc

    (push-byte line)
    (update-in acc [:state :pushes] conj (push-byte line))

    ;; First literal encode_u32 after the opcode byte is a GC/sat-trunc
    ;; subopcode (0xFB/0xFC families); anything else is payload and stops
    ;; opcode collection.
    (u32-lit line)
    (-> acc
        (update-in [:state :const] #(or % (u32-lit line)))
        (assoc-in [:state :stopped] true))

    :else
    (assoc-in acc [:state :stopped] true)))

(defn encoder-arms
  "Returns {variant-name opcode-string} for every arm before `_ =>`."
  []
  (let [lines (->> (slurp encoder-path)
                   str/split-lines
                   (drop-while #(not (str/includes? % "fn encode_instr")))
                   (take-while #(not (re-find #"^fn encode_i32\b" %))))]
    (->> lines
         (reduce step {:table [] :current nil :state nil})
         flush-arm
         :table
         (into {}))))

;; --- 3. Mnemonic derivation --------------------------------------------------

(def prefixes
  ["I32" "I64" "F32" "F64" "Local" "Global" "Memory" "Struct" "Array"
   "Ref" "Extern" "Any"])

;; Wire mnemonics the mechanical rule cannot derive.
(def overrides
  {"F64ConvertSI32" "f64.convert_i32_s" ; operand order flips on the wire
   "Throw"          "throw_ref"})       ; encoder emits subopcode 0x08

(defn- snake [s]
  (-> s
      (str/replace #"([a-z0-9])([A-Z])" "$1_$2")
      (str/replace #"([A-Z]+)([A-Z][a-z])" "$1_$2")
      str/lower-case))

(defn mnemonic [v]
  (if-let [o (overrides v)]
    o
    (if-let [p (some #(when (and (str/starts-with? v %)
                                 (< (count %) (count v)))
                        %)
                     prefixes)]
      (str (str/lower-case p) "." (snake (subs v (count p))))
      (snake v))))

;; --- 4. Render ----------------------------------------------------------------

(defn- header [encoded-n total-n]
  ["# Supported Wasm instruction matrix"
   ""
   (str "<!-- GENERATED FILE — do not edit by hand. Regenerate with:"
        " `bb scripts/gen_supported_spec.clj` -->")
   ""
   (str "Generated from the `Instr` enum (`crates/gleam-wasm/src/ir/types.rs`)"
        " and the binary encoder's opcode table"
        " (`crates/gleam-wasm/src/emit/wasm/encoder.rs`, `encode_instr`).")
   ""
   (format "**%d of %d** `Instr` variants reach spec bytes in the binary encoder;"
           encoded-n total-n)
   (str "the rest fall through the `_ =>` arm and encode as `unreachable` (0x00)."
        " The WAT text backend (`crates/gleam-wasm/src/ir/function.rs` Display)"
        " is separate; this matrix is authoritative for `.wasm` output only.")
   ""
   "Opcode column: leading bytes of the encoding. `0xFB`/`0xFC` prefixes show"
   " their subopcode; operands (LEB128 indices/immediates, memarg align+offset)"
   " follow on the wire and are not listed."
   ""
   "| # | `Instr` variant | mnemonic | opcode |"
   "|---|-----------------|----------|--------|"])

(defn -main [& _]
  (let [vs (enum-variants)
        arms (encoder-arms)
        encoded (filterv #(get arms %) vs)
        rows (map-indexed
              (fn [i v]
                (if-let [op (get arms v)]
                  (format "| %d | `%s` | `%s` | `%s` |" (inc i) v (mnemonic v) op)
                  (format "| %d | `%s` | `%s` | ⛔ unreachable fallback |"
                          (inc i) v (mnemonic v))))
              vs)]
    (spit out-path (str (str/join "\n" (concat (header (count encoded) (count vs))
                                               rows))
                        "\n"))
    (println (format "wrote %s: %d/%d variants encoded"
                     out-path (count encoded) (count vs)))))

(-main)
