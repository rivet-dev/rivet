var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/free-brands-svg-icons/faSolana.js
var require_faSolana = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSolana.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "solana";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e85e";
    var svgPathData = "M510.5 385.2l-84.9 88.7c-1.8 1.9-4.1 3.5-6.6 4.5s-5.2 1.6-7.9 1.6L8.9 480c-1.9 0-3.8-.5-5.4-1.6s-2.9-2.5-3.6-4.2-1-3.6-.7-5.5 1.2-3.6 2.5-4.9l84.9-88.7c1.8-1.9 4.1-3.5 6.5-4.5s5.1-1.6 7.8-1.6l402.4 0c1.9 0 3.8 .5 5.4 1.6s2.9 2.5 3.6 4.2 1 3.6 .7 5.5-1.2 3.6-2.5 4.9zM425.7 206.6c-1.8-1.9-4.1-3.5-6.6-4.5s-5.2-1.6-7.9-1.6l-402.4 0c-1.9 0-3.8 .5-5.4 1.6s-2.9 2.5-3.6 4.2-1 3.6-.7 5.5 1.2 3.6 2.5 4.9l84.9 88.7c1.8 1.9 4.1 3.5 6.5 4.5s5.1 1.6 7.8 1.6l402.4 0c1.9 0 3.8-.5 5.4-1.6s2.9-2.5 3.6-4.2 1-3.6 .7-5.5-1.2-3.6-2.5-4.9l-84.9-88.7zM8.9 142.9l402.4 0c2.7 0 5.4-.5 7.9-1.6s4.7-2.6 6.6-4.5l84.9-88.7c1.3-1.4 2.2-3.1 2.5-4.9s.1-3.7-.7-5.5-2-3.2-3.6-4.2-3.5-1.6-5.4-1.6L101 32c-2.7 0-5.4 .5-7.8 1.6s-4.7 2.6-6.5 4.5L1.7 126.8c-1.3 1.4-2.2 3.1-2.5 4.9s-.1 3.7 .7 5.5 2 3.2 3.6 4.2 3.5 1.6 5.4 1.6z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faSolana = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faSolana.js
var import_faSolana = __toESM(require_faSolana());
var export_faSolana = import_faSolana.definition;
export {
  export_faSolana as faSolana
};
