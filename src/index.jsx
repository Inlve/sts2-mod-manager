// 纯 Tauri 入口：先把 window.api 桥接装好，再挂载 React
import './tauri-api';
import { createRoot } from 'react-dom/client';
import App from './App';
import './index.css';

createRoot(document.getElementById('root')).render(<App />);
