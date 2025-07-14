import React, { useState, useEffect, useMemo, useCallback, useRef, useContext } from 'react';

// For testing namespaced elements
const SVG = {
  Circle: 'circle',
};

// Test Context
const ThemeContext = React.createContext('light');

/**
 * A standard class component.
 * It's important to test these as well.
 */
export class ClassComponent extends React.Component<{ title: string }> {
  render() {
    // A JSX comment
    return <h1>{this.props.title}</h1>;
  }
}

// A custom hook for a counter
function useCounter(initialValue = 0) {
  const [count, setCount] = useState(initialValue);
  const increment = () => setCount(c => c + 1);
  return { count, increment };
}

// Another custom hook with dependencies
const useWindowWidth = () => {
  const [width, setWidth] = useState(window.innerWidth);
  useEffect(() => {
    const handleResize = () => setWidth(window.innerWidth);
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []); // Dependency array
  return width;
};

// A functional component using React.memo
export const MemoizedComponent = React.memo(function MemoizedComponent({ data }) {
  const calculated = useMemo(() => {
    // Some expensive calculation
    return data.id * 2;
  }, [data.id]);

  return <p>Memoized: {calculated}</p>;
});


// The main comprehensive component for testing
function ComprehensiveComponent(props) {
  const { count, increment } = useCounter(10);
  const width = useWindowWidth();
  const inputRef = useRef<HTMLInputElement>(null);
  const theme = useContext(ThemeContext);

  const handleClick = useCallback(() => {
    if (inputRef.current) {
      inputRef.current.focus();
    }
    increment();
  }, [increment]);

  return (
    <div className={`theme-${theme}`} style={{ border: '1px solid black' }}>
      <ClassComponent title="Class Component Title" />
      <MemoizedComponent data={{ id: 5 }} />
      <section>
        <label htmlFor="test-input">Focus Test</label>
        <input ref={inputRef} id="test-input" type="text" />
      </section>
      <button onClick={handleClick} disabled={false}>
        Increment and Focus
      </button>
      <span>Width: {width}</span>
      <p>Current Count: {count}</p>
      <svg width="100" height="100">
        <SVG.Circle cx="50" cy="50" r="40" stroke="green" fill="yellow" />
      </svg>
    </div>
  );
}

export default ComprehensiveComponent;
