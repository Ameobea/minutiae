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
  define __SELF_ACTIONS (
    concat __SELF_ACTIONS '(action)
  )
))

; Action structs

(struct Translation (
  (x integer)
  (y integer)
))

(struct Suicide ())

; Action dispatchers

(define (translate x y) (
  push_self_action (new Translation :x x :y y)
))

(define (suicide) (
  push_self_action (new Suicide)
))
