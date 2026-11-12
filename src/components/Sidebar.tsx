import type React from "react";
import { useState, useMemo } from "react";
import { Button } from "@/components/ui/button";
import { ChevronRight, ChevronDown, Search } from "lucide-react";
import Fuse from "fuse.js";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

export interface SidebarItem {
  id: string;
  name: string;
  description: string;
  link?: string;
  tags: string[];
}

interface SidebarProps {
  operations: SidebarItem[];
  onOperationSelect: (operation: SidebarItem) => void;
  /** On narrow screens the sidebar is a drawer; this says whether it is showing. */
  open: boolean;
}

const ALL = "All";

function OperationButton({
  operation,
  onSelect,
}: {
  operation: SidebarItem;
  onSelect: (operation: SidebarItem) => void;
}) {
  return (
    <TooltipProvider>
      <Tooltip delayDuration={200}>
        <TooltipTrigger asChild>
          <Button
            className="w-full justify-start text-sm"
            variant="ghost"
            onClick={() => onSelect(operation)}
            aria-label={`Add ${operation.name} operation`}
          >
            {operation.name}
          </Button>
        </TooltipTrigger>
        <TooltipContent
          className="max-w-sm rounded-md border border-zinc-800 bg-zinc-900 px-3 py-2 text-zinc-50 shadow-lg"
          sideOffset={5}
          side="right"
        >
          <p className="mb-2 text-sm">{operation.description}</p>
          {operation.link && (
            <a
              href={operation.link}
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm text-blue-400 underline hover:text-blue-300"
              onClick={(e) => e.stopPropagation()}
            >
              Learn more
            </a>
          )}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

const Sidebar: React.FC<SidebarProps> = ({ operations, onOperationSelect, open }) => {
  const [expandedTags, setExpandedTags] = useState<string[]>([]);
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

  const toggleTag = (tag: string) => {
    setExpandedTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag]
    );
  };

  const groupedOperations = useMemo(() => {
    const groups: Record<string, SidebarItem[]> = { [ALL]: operations };
    for (const operation of operations) {
      for (const tag of operation.tags) {
        (groups[tag] ??= []).push(operation);
      }
    }
    return groups;
  }, [operations]);

  return (
    <nav
      id="operations"
      className={`h-full w-60 shrink-0 overflow-y-auto border-r bg-white p-4 ${
        open ? "max-md:fixed max-md:inset-y-0 max-md:left-0 max-md:z-40 max-md:shadow-xl" : "max-md:hidden"
      }`}
      aria-label="Operations"
    >
      <h1 className="text-lg font-bold">Cyber Nohut</h1>
      <h2 className="mb-3 mt-4 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        Operations
      </h2>

      <div className="relative mb-4">
        <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-600" />
        <input
          type="text"
          placeholder="Search operations..."
          className="w-full pl-8 pr-4 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          aria-label="Search operations"
        />
      </div>

      {searchQuery && searchResults.length > 0 && (
        <div className="mb-4">
          <h3 className="text-sm font-semibold text-gray-700 mb-2">
            Search Results
          </h3>
          <div className="space-y-1">
            {searchResults.map((op) => (
              <OperationButton key={op.id} operation={op} onSelect={onOperationSelect} />
            ))}
          </div>
        </div>
      )}

      {Object.entries(groupedOperations).map(([tag, ops]) => (
        <div key={tag} className="mb-2">
          <Button
            variant="ghost"
            className="w-full justify-between font-semibold"
            aria-expanded={expandedTags.includes(tag)}
            onClick={() => toggleTag(tag)}
          >
            <span>
              {tag} <span className="font-normal text-muted-foreground">{ops.length}</span>
            </span>
            {expandedTags.includes(tag) ? (
              <ChevronDown className="h-4 w-4" />
            ) : (
              <ChevronRight className="h-4 w-4" />
            )}
          </Button>
          {expandedTags.includes(tag) && (
            <div className="ml-3 mt-1 space-y-0.5">
              {ops.map((op) => (
                <OperationButton key={op.id} operation={op} onSelect={onOperationSelect} />
              ))}
            </div>
          )}
        </div>
      ))}
    </nav>
  );
};

export default Sidebar;
