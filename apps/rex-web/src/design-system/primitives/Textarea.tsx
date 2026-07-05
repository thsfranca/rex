export interface SegmentedControlProps<T extends string> {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  disabled?: boolean;
  testId?: string;
}

export function SegmentedControl<T extends string>({
  value,
  options,
  onChange,
  disabled,
  testId,
}: SegmentedControlProps<T>) {
  return (
    <div className="rex-segmented" data-testid={testId} role="group">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          className={`rex-segmented__item${value === option.value ? " rex-segmented__item--active" : ""}`}
          disabled={disabled}
          aria-pressed={value === option.value}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

export interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  autoResize?: boolean;
}

export function Textarea({ autoResize = false, onChange, value, ...rest }: TextareaProps) {
  function handleChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
    if (autoResize) {
      e.target.style.height = "auto";
      e.target.style.height = `${e.target.scrollHeight}px`;
    }
    onChange?.(e);
  }

  return (
    <textarea
      className="rex-textarea"
      value={value}
      onChange={handleChange}
      {...rest}
    />
  );
}
