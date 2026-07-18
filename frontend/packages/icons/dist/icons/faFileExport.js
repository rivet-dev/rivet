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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFileExport.js
var require_faFileExport = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFileExport.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-export";
    var width = 576;
    var height = 512;
    var aliases = ["arrow-right-from-file"];
    var unicode = "f56e";
    var svgPathData = "M96.5 0c-35.3 0-64 28.7-64 64l0 384c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-96 78.1 0-31 31c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l72-72c9.4-9.4 9.4-24.6 0-33.9l-72-72c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l31 31-78.1 0 0-133.5c0-17-6.7-33.3-18.7-45.3L291.2 18.7C279.2 6.7 263 0 246 0L96.5 0zM358 176l-93.5 0c-13.3 0-24-10.7-24-24L240.5 58.5 358 176zM224.5 328c0-13.3 10.7-24 24-24l104 0 0 48-104 0c-13.3 0-24-10.7-24-24z";
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
    exports.faFileExport = exports.definition;
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

// src/_entries/faFileExport.js
var import_faFileExport = __toESM(require_faFileExport());
var export_faFileExport = import_faFileExport.definition;
export {
  export_faFileExport as faFileExport
};
