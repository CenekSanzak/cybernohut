import React, { useRef } from "react";
import { Download, Play, Upload, Share2, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  deserialize,
  serialize,
  toShareUrl,
  type GraphSnapshot,
} from "@/graph/persistence";
import { copyToClipboard } from "@/utils/clipboard";
import { showNotification } from "@/utils/notifications";

interface GraphControlsProps {
  state: GraphSnapshot;
  onImport: (snapshot: GraphSnapshot) => void;
  onClear: () => void;
  autoCalculate: boolean;
  onAutoCalculateChange: (enabled: boolean) => void;
  /** Runs the graph once; used when auto-calculate is off. */
  onRun: () => void;
}

export const GraphControls: React.FC<GraphControlsProps> = ({
  state,
  onImport,
  onClear,
  autoCalculate,
  onAutoCalculateChange,
  onRun,
}) => {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleExport = () => {
    try {
      const jsonString = serialize(state);
      const blob = new Blob([jsonString], { type: "application/json" });
      const url = URL.createObjectURL(blob);

      const timestamp = new Date()
        .toISOString()
        .replace(/[:.]/g, "-")
        .slice(0, 19);
      const filename = `cybernohut-graph-${timestamp}.json`;

      const link = document.createElement("a");
      link.href = url;
      link.download = filename;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);

      showNotification("Graph exported successfully!", "success");
    } catch (error) {
      console.error("Export failed:", error);
      showNotification("Failed to export graph", "error");
    }
  };

  const handleImport = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = () => {
      const snapshot = deserialize(String(reader.result));
      if (snapshot && snapshot.nodes.length > 0) {
        onImport(snapshot);
        showNotification("Graph imported successfully!", "success");
      } else {
        showNotification("Invalid graph file", "error");
      }
    };

    reader.readAsText(file);
    // Reset input
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  };

  const handleShare = async () => {
    try {
      const shareableURL = toShareUrl(state);
      const success = await copyToClipboard(shareableURL);

      if (success) {
        showNotification("Link copied to clipboard!", "success");
        // Update browser URL
        window.history.pushState({}, "", shareableURL);
      } else {
        showNotification("Failed to copy link", "error");
      }
    } catch (error) {
      console.error("Share failed:", error);
      showNotification("Failed to create shareable link", "error");
    }
  };

  const handleClear = () => {
    if (
      window.confirm(
        "Are you sure you want to clear the current graph? This action cannot be undone."
      )
    ) {
      onClear();
      showNotification("Graph cleared", "info");
    }
  };

  return (
    <div className="flex flex-wrap items-center gap-2 border-b bg-white p-2">
      <Button
        variant="outline"
        size="sm"
        onClick={handleExport}
        className="flex items-center gap-2"
      >
        <Download size={16} />
        Export
      </Button>

      <Button
        variant="outline"
        size="sm"
        onClick={() => fileInputRef.current?.click()}
        className="flex items-center gap-2"
      >
        <Upload size={16} />
        Import
      </Button>

      <input
        ref={fileInputRef}
        type="file"
        accept=".json"
        onChange={handleImport}
        className="hidden"
        aria-label="Import graph file"
      />

      <Button
        variant="outline"
        size="sm"
        onClick={handleShare}
        className="flex items-center gap-2"
      >
        <Share2 size={16} />
        Share
      </Button>

      <Button
        variant="outline"
        size="sm"
        onClick={handleClear}
        className="flex items-center gap-2 text-red-600 hover:text-red-700 hover:bg-red-50"
      >
        <Trash2 size={16} />
        Clear
      </Button>

      <div className="ml-auto flex items-center gap-3">
        {!autoCalculate && (
          <Button size="sm" onClick={onRun}>
            <Play size={16} />
            Run
          </Button>
        )}
        <label
          className="flex items-center gap-2 text-sm"
          title="Re-run the graph whenever it changes. Turn off for very large inputs."
        >
          <input
            type="checkbox"
            checked={autoCalculate}
            onChange={(event) => onAutoCalculateChange(event.target.checked)}
          />
          Auto-calculate
        </label>
        <a href="bench.html" className="text-sm text-muted-foreground underline hover:text-foreground">
          Benchmark
        </a>
      </div>
    </div>
  );
};
