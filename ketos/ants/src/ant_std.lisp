;;; Declares standard functions that are loaded into the context of ant drivers

(use random (random))

; Initialize global action buffers
(define __CELL_ACTIONS ())
(define __SELF_ACTIONS ())
(define __ENTITY_ACTIONS ())

(define (push-self-action action) (
  define __SELF_ACTIONS (append __SELF_ACTIONS action)
))

; Action dispatchers

(define (translate x y) (
  push-self-action `("translate" ,x ,y)
))

(define (suicide) (
  push-self-action `("suicide")
))

; Calls the provided lambda with the provided arguments `n` times
(define (do-n n function :rest args) (
  if (> n 0) (
    do
      (apply function args)
      (apply do-n (- n 1) function args)
    )
  )
)

; functional programming core functions

(define (partial function :rest rest) (
  lambda (:rest args) (
    apply function (concat rest args)
  )
))

(define (reduce list reducer :optional acc)(
  if (> (len list) 0) (
    reduce (tail list) reducer (reducer acc (first list))
  )
  acc
))

(define (map list function) (
  reduce list (lambda (acc item) (append acc (function item)))
))

(define (for-each list function) (
  reduce list (lambda (acc item) (do (function item) ()))
))

; utility functions

(define (manhattan-distance x1 y1 x2 y2) (
  let (
    (x-diff (if (< x2 x1) (- x2 x1) (- x1 x2)))
    (y-diff (if (< y2 y1) (- y2 y1) (- y1 y2)))
  )(
    + x-diff y-diff
  )
))

; Returns a random integer from `from` to `to`, bottom inclusive top exclusive
(define (random-int from to)(
  + (int (* (random) (- to from))) from
))

(define (coord-to-universe-index x y universe-size)(
  (+ (* y universe-size) x)
))

; Given a universe index and the univers size, returns the corresponding coordinate as `(x, y)`.
(define (universe-index-to-coords universe-index universe-size)(
  let (
    (rows (/ universe-index universe-size))
  )`(
      ,(int (* (fract (float rows)) universe-size))
      ,(int (floor rows))
  )
))

(define (print-int label num)(
  println (format (concat label ": ~d") num))
)

(define (is-valid-coord x y universe-size)(
  and
    (and (>= x 0) (>= y 0))
    (and (< x universe-size) (< y universe-size))
))
