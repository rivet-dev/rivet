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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFileWaveform.js
var require_faFileWaveform = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFileWaveform.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-waveform";
    var width = 384;
    var height = 512;
    var aliases = ["file-medical-alt"];
    var unicode = "f478";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L213.5 0c17 0 33.3 6.7 45.3 18.7L365.3 125.3c12 12 18.7 28.3 18.7 45.3L384 448c0 35.3-28.7 64-64 64L64 512c-35.3 0-64-28.7-64-64l0-96 60.3 0 48.8 62.7c5.1 6.6 13.3 10 21.6 9.1s15.5-6.1 19.1-13.6l42.9-91.2 9.9 19.8c4.1 8.1 12.4 13.3 21.5 13.3l72 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-57.2 0-25.4-50.7c-4.1-8.2-12.6-13.4-21.8-13.3s-17.5 5.5-21.4 13.8l-47.3 100.6-32-41.1C86.4 307.4 79.4 304 72 304L0 304 0 64zm208-5.5l0 93.5c0 13.3 10.7 24 24 24L325.5 176 208 58.5z";
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
    exports.faFileWaveform = exports.definition;
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

// src/_entries/faFileWaveform.js
var import_faFileWaveform = __toESM(require_faFileWaveform());
var export_faFileWaveform = import_faFileWaveform.definition;
export {
  export_faFileWaveform as faFileWaveform
};
