# Changelog

## [0.2.0](https://github.com/Quasiflo/nemasvg/compare/v0.1.0...v0.2.0) (2026-09-16)


### Features

* convert asciicast v3 recordings to animated SVG ([d4467c7](https://github.com/Quasiflo/nemasvg/commit/d4467c7bbc558ab60fe11285f488e2b42392cab1))
* embed subset fonts by default, monochrome emoji opt-in ([bc37f0c](https://github.com/Quasiflo/nemasvg/commit/bc37f0c08da043f33e98bdd3cc87ea0e23906587))
* harden and simplify MVP pipeline ([6254431](https://github.com/Quasiflo/nemasvg/commit/62544317b2e93635b3f588b8668d38fbdc3f36ac))


### Bug Fixes

* add white-space:pre so Chromium keeps edge spaces ([92c0b11](https://github.com/Quasiflo/nemasvg/commit/92c0b11a2d18651dfade26897f2a6a81499ee886))
* anchor text runs to grid, clarify cursor and scroll fixtures ([a30075c](https://github.com/Quasiflo/nemasvg/commit/a30075c081f047bdeaa98caa95eb506ff595171d))
* **deps:** update cargo dependencies ([#6](https://github.com/Quasiflo/nemasvg/issues/6)) ([f096de9](https://github.com/Quasiflo/nemasvg/commit/f096de9feb8d38962b99e93895e141fac1d937f7))
* replace textLength with letter-spacing grid compensation ([df86455](https://github.com/Quasiflo/nemasvg/commit/df86455ebd286b55c938c0039ba7f10aa01c4ea6))
* scope xml:space to SVG root so defs keep whitespace ([5e48f99](https://github.com/Quasiflo/nemasvg/commit/5e48f9937bff933c7cba535918403f04a15bb57a))


### Performance Improvements

* intern repeated runs inside rows via exact byte economics ([dcc40af](https://github.com/Quasiflo/nemasvg/commit/dcc40afe6ca6059ddd38b51e79fa262a9104faa9))
