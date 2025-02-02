import type React from "react";
import { useState, useMemo } from "react";
import { Button } from "@/components/ui/button";
import { type Operation, OperationTags } from "@/operations/types";
import { ChevronRight, ChevronDown, Search } from "lucide-react";
import Fuse from "fuse.js";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface SidebarProps {
  operations: Operation[];
  onOperationSelect: (operation: Operation) => void;
}

const Sidebar: React.FC<SidebarProps> = ({ operations, onOperationSelect }) => {
  const [expandedTags, setExpandedTags] = useState<OperationTags[]>([]);
  const [searchQuery, setSearchQuery] = useState("");

  const fuse = useMemo(
    () =>
      new Fuse(operations, {
        keys: ["name", "description", "tags"],
        threshold: 0.4,
      }),
    [operations]
  );

  const searchResults = useMemo(() => {
    if (!searchQuery) return [];
    return fuse
      .search(searchQuery)
      .slice(0, 5)
      .map((result) => result.item);
  }, [searchQuery, fuse]);

  const toggleTag = (tag: OperationTags) => {
    setExpandedTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag]
    );
  };

  const groupedOperations = useMemo(() => {
    return Object.values(OperationTags).reduce((acc, tag) => {
      const filteredOps = operations.filter((op) => op.tags.includes(tag));
      if (filteredOps.length > 0) {
        acc[tag] = filteredOps;
      }
      return acc;
    }, {} as Record<OperationTags, Operation[]>);
  }, [operations]);

  return (
    <div className="w-64 bg-white shadow-md p-4 h-screen overflow-y-auto">
      <h2 className="text-xl font-bold mb-4">Operations</h2>

      <div className="relative mb-4">
        <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
        <input
          type="text"
          placeholder="Search operations..."
          className="w-full pl-8 pr-4 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
      </div>

      {searchQuery && searchResults.length > 0 && (
        <div className="mb-4">
          <h3 className="text-sm font-semibold text-gray-500 mb-2">
            Search Results
          </h3>
          <div className="space-y-1">
            {searchResults.map((op) => (
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
        </div>
      )}

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
