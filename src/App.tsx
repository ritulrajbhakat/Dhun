import React, { useEffect } from 'react';
import { useDhunStore } from './store/useDhunStore';
import { StageDisplayScreen } from './components/stage/StageDisplayScreen';

export const App: React.FC = () => {
  const { refreshAll } = useDhunStore();

  useEffect(() => {
    refreshAll();
  }, [refreshAll]);

  return (
    <div className="h-screen w-screen overflow-hidden bg-black select-none font-sans">
      <StageDisplayScreen />
    </div>
  );
};
