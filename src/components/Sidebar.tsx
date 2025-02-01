import type React from "react";
import { useState } from "react";
import { Button } from "./ui/button";
import { type Operation, OperationTags } from "../operations/types";
import { ChevronRight, ChevronDown } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./ui/tooltip";

interface SidebarProps {
  operations: Operation[];
  onOperationSelect: (operation: Operation) => void;
}

const Sidebar: React.FC<SidebarProps> = ({ operations, onOperationSelect }) => {
  const [expandedTags, setExpandedTags] = useState<OperationTags[]>([]);

  const toggleTag = (tag: OperationTags) => {
    setExpandedTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag]
    );
  };

  const groupedOperations = Object.values(OperationTags).reduce((acc, tag) => {
    const filteredOps = operations.filter((op) => op.tags.includes(tag));
    if (filteredOps.length > 0) {
      acc[tag] = filteredOps;
    }
    return acc;
  }, {} as Record<OperationTags, Operation[]>);

  return (
    <div className="w-64 bg-white shadow-md p-4 h-screen overflow-y-auto">
      <h2 className="text-xl font-bold mb-4">Operations</h2>
      {Object.entries(groupedOperations).map(([tag, ops]) => (
        <div key={tag} className="mb-2">
          <Button
            variant="ghost"
            className="w-full justify-between font-semibold"
            onClick={() => toggleTag(tag as OperationTags)}
          >
            {tag}
            {expandedTags.includes(tag as OperationTags) ? (
              <ChevronDown className="h-4 w-4" />
            ) : (
              <ChevronRight className="h-4 w-4" />
            )}
          </Button>
          {expandedTags.includes(tag as OperationTags) && (
            <div className="ml-4 mt-2 space-y-2">
              {ops.map((op) => (
                <TooltipProvider key={op.id}>
                  <Tooltip delayDuration={200}>
                    <TooltipTrigger asChild>
                      <Button
                        className="w-full justify-start text-sm"
                        variant="ghost"
                        onClick={() => onOperationSelect(op)}
                      >
                        {op.name}
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent
                      className="bg-zinc-900 text-zinc-50 px-3 py-2 rounded-md shadow-lg border border-zinc-800 max-w-sm"
                      sideOffset={5}
                      side="right"
                    >
                      <p className="text-sm mb-2">
                        {op.description || "No description available"}
                      </p>
                      {op.link && (
                        <a
                          href={op.link}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-sm text-blue-400 hover:text-blue-300 underline"
                          onClick={(e) => e.stopPropagation()}
                        >
                          Learn more
                        </a>
                      )}
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
};

export default Sidebar;
