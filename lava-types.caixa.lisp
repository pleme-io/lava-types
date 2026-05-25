(defcaixa
  :name
  "lava-types"
  :kind
  :Biblioteca
  :ecosystem
  :rust-single-crate
  :package
  {:name "lava-types"
   :version "0.1.0"
   :description "Typed constraint validators for lava architectures. Pangea Dry::Struct analog: CIDR / port / protocol / enum / regex / length / range / IPv4 / IPv6 / hostname. Validated at composition time (not apply time) — invalid CIDR fails the plan, not the cloud API. Pangea Types::String.constrained(...) → lava (:type :cidr-block) etc."
   :license "MIT"
   :repository "https://github.com/pleme-io/lava-types"}
  :ci-config
  {:bump {:default-type "patch"}
   :publish {:no-verify true}}
  :workflows
  [:auto-release :pre-merge-gate :security-gate])
