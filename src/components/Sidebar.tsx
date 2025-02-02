import React, { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Operation } from "@/operations/types";

interface SidebarProps {
  operations: Operation[];
  onOperationSelect: (operation: Operation) => void;
}

const Sidebar = React.memo<SidebarProps>(
  ({ operations, onOperationSelect }) => {
    const parentRef = useRef<HTMLDivElement>(null);

    const virtualizer = useVirtualizer({
      count: operations.length,
      getScrollElement: () => parentRef.current,
      estimateSize: () => 60,
      overscan: 5,
    });

    return (
      <div className="w-1/4 p-4 border-r bg-gray-50">
        <h2 className="text-xl font-bold mb-4">Operations</h2>
        <div ref={parentRef} className="h-[calc(100vh-8rem)] overflow-auto">
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {virtualizer.getVirtualItems().map((virtualRow) => (
              <div
                key={virtualRow.key}
                data-index={virtualRow.index}
                className={`absolute top-0 left-0 w-full`}
                style={{
                  height: `${virtualRow.size}px`,
                  transform: `translateY(${virtualRow.start}px)`,
                }}
              >
                <button
                  className="w-full p-4 text-left hover:bg-gray-100 rounded transition-colors"
                  onClick={() =>
                    onOperationSelect(operations[virtualRow.index])
                  }
                >
                  <div className="font-medium">
                    {operations[virtualRow.index].name}
                  </div>
                  <div className="text-sm text-gray-500">
                    {operations[virtualRow.index].description}
                  </div>
                </button>
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }
);

Sidebar.displayName = "Sidebar";

export default Sidebar;
