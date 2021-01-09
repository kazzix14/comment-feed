import './static/style.scss';
import './semantic/dist/semantic.css';
import './semantic/dist/semantic.js';

import("./pkg").then(module => {
  module.run_app();
});
