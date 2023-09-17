;; What follows is a "manifest" equivalent to the command line you gave.
;; You can store it in a file that you may then pass to any 'guix' command
;; that accepts a '--manifest' (or '-m') option.

(concatenate-manifests
 (list
  (packages->manifest
   (list `(,(@@ (binary packages rust) rust-stable-1.71.1-x86_64-linux) "rust-analyzer-preview")
	 `(,(@@ (binary packages rust) rust-stable-1.71.1-x86_64-linux) "rustfmt-preview")
	 `(,(@@ (binary packages rust) rust-stable-1.71.1-x86_64-linux) "cargo")
	 (@@ (binary packages rust) rust-stable-1.71.1-x86_64-linux)
	 ))
  (specifications->manifest
   '("clang-toolchain"
     "pkg-config"
     "mesa"
     "libxkbcommon"
     ;; compositor
     "expat"
     ;; wrench font-loader requires this
     "fontconfig"
     "freetype"

     "wayland"))))
