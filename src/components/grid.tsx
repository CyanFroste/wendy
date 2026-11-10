import { useEffect, useRef, useState } from 'react'
import { ReactGridLayout, noCompactor } from 'react-grid-layout'
import { GridBackground } from 'react-grid-layout/extras'
import type { ResizeHandle as ResizeHandleFn } from 'react-grid-layout'

type GridProps = {
  containerWidth: number
  containerHeight: number
  cols: number
  rows: number
  gap: number
  padding: number
}

function Grid({ containerWidth, containerHeight, cols, rows, gap, padding }: GridProps) {
  const rowHeight = (containerHeight - gap * (rows - 1)) / rows

  return (
    <>
      <GridBackground
        width={containerWidth}
        cols={cols}
        rows={rows}
        rowHeight={rowHeight}
        margin={[gap, gap]}
        containerPadding={[0, 0]}
        borderRadius={0}
        color="rgba(255, 255, 255, 0.5)"
        style={{ top: padding, left: padding }}
      />

      <ReactGridLayout
        autoSize={false}
        width={containerWidth}
        compactor={noCompactor}
        dragConfig={{ enabled: true, handle: '.handle' }}
        resizeConfig={{ enabled: true, handleComponent: ResizeHandle }}
        gridConfig={{ cols, rowHeight, maxRows: rows, margin: [gap, gap], containerPadding: [0, 0] }}
        layout={[
          { i: '0', x: 0, y: 0, w: 1, h: 1 },
          { i: '1', x: 1, y: 0, w: 1, h: 1 },
          { i: '2', x: 2, y: 0, w: 1, h: 1 },
          { i: '3', x: 3, y: 0, w: 1, h: 1 },
          { i: '4', x: 4, y: 0, w: 1, h: 1 },
          { i: '5', x: 5, y: 0, w: 1, h: 1 },
        ]}>
        <div key="0">0</div>
        <div key="1">1</div>
        <div key="2">2</div>
        <div key="3">3</div>
        <div key="4">4</div>
        <div key="5">5</div>
      </ReactGridLayout>
    </>
  )
}

const ResizeHandle: ResizeHandleFn = (axis, ref) => {
  // axis as "se" for now
  return (
    <span ref={ref} className="absolute bottom-0 right-0" style={{ cursor: `${axis}-resize` }}>
      resize
    </span>
  )
}

type GridContainerProps = { offset: number; hackOffset: number }

export function GridContainer({ offset, hackOffset }: GridContainerProps) {
  const containerRef = useRef<HTMLDivElement>(null)

  const [containerWidth, setContainerWidth] = useState(0)
  const [containerHeight, setContainerHeight] = useState(0)

  const padding = 10
  const cols = 28
  const rows = 16
  const gap = 1

  useEffect(() => {
    const resizeObserver = new ResizeObserver(([entry]) => {
      if (entry.contentBoxSize) {
        const { inlineSize: w, blockSize: h } = entry.contentBoxSize[0]

        setContainerWidth(w)
        setContainerHeight(h)
      }
    })

    if (containerRef.current) resizeObserver.observe(containerRef.current)

    return () => {
      if (containerRef.current) resizeObserver.unobserve(containerRef.current)
      resizeObserver.disconnect()
    }
  }, [])

  return (
    <div className="w-screen h-screen bg-red-500 flex flex-col">
      <div className="shrink-0 bg-blue-500 w-full" style={{ height: `${hackOffset}px` }} />

      <div
        ref={containerRef}
        className="w-full overflow-hidden relative"
        style={{ height: `calc(100% - ${offset}px)`, padding }}>
        {containerWidth && containerHeight && (
          <Grid
            containerWidth={containerWidth}
            containerHeight={containerHeight}
            padding={padding}
            cols={cols}
            rows={rows}
            gap={gap}
          />
        )}
      </div>
    </div>
  )
}
