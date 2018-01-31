;;; Declares standard functions that are loaded into the context of ant drivers

(define (manhattan_distance x1 y1 x2 y2) (
  let (
    (x_diff (if (< x2 x1) (- x2 x1) (- x1 x2)))
    (y_diff (if (< y2 y1) (- y2 y1) (- y1 y2)))
  )(
    + x_diff y_diff
  )
))

; Initialize global action buffers
(define __CELL_ACTIONS ())
(define __SELF_ACTIONS ())
(define __ENTITY_ACTIONS ())

(define (push_self_action action) (
  define __SELF_ACTIONS (append __SELF_ACTIONS action)
))

; Action dispatchers

(define (translate x y) (
  push_self_action `("translate" ,x ,y)
))

(define (suicide) (
  push_self_action `("suicide")
))

;;; Calls the provided lambda with the provided arguments `n` times
(define (do-n n function :rest args) (
  if (> n 0) (
    do
      (apply function args)
      (apply do-n (- n 1) function args)
    )
  )
)

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
