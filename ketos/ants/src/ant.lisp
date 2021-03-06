(
  let (
    (coords (universe-index-to-coords UNIVERSE_INDEX UNIVERSE_SIZE))
  )(
    let (
      (x (first coords))
      (y (last coords))
      (x-offset (random-int -1 2))
      (y-offset (random-int -1 2))
    )(
      let (
        (new-x (+ x x-offset))
        (new-y (+ y y-offset))
      )(
        if (is-valid-coord new-x new-y UNIVERSE_SIZE)
          (translate x-offset y-offset)
      )
    )
  )
)
