import type React from "react";
import { Textarea } from "./ui/textarea";

interface InputOutputProps {
  input: string;
  output: string;
  onInputChange: (value: string) => void;
  calculate: () => void;
}

const InputOutput: React.FC<InputOutputProps> = ({
  input,
  output,
  onInputChange,
  calculate,
}) => {
  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 p-4">
        <h3 className="text-lg font-semibold mb-2">Input</h3>
        <Textarea
          value={input}
          onChange={(e) => onInputChange(e.target.value)}
          className="w-full h-full min-h-[200px]"
          placeholder="Enter your input here..."
        />
      </div>
      <div className="flex-1 p-4 bg-gray-50">
        <h3 className="text-lg font-semibold mb-2 flex items-center gap-2">
          Output{" "}
          <button
            onClick={calculate}
            className="px-4 py-1 bg-blue-500 text-white rounded-full hover:bg-blue-600 transition-colors"
          >
            Calculate
          </button>
        </h3>
        <Textarea
          value={output}
          readOnly
          className="w-full h-full min-h-[200px] bg-white"
          placeholder="Output will appear here..."
        />
      </div>
    </div>
  );
};

export default InputOutput;
