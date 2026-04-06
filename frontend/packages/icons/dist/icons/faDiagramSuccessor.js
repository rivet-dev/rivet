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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDiagramSuccessor.js
var require_faDiagramSuccessor = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDiagramSuccessor.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "diagram-successor";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e47a";
    var svgPathData = "M289.2 137.9c2.5-6 8.3-9.9 14.8-9.9l40 0 0-24c0-13.3-10.7-24-24-24l-98 0c1.3 5.1 2 10.5 2 16l0 64c0 35.3-28.7 64-64 64l-96 0c-35.3 0-64-28.7-64-64L0 96C0 60.7 28.7 32 64 32l256 0c39.8 0 72 32.2 72 72l0 24 40 0c6.5 0 12.3 3.9 14.8 9.9s1.1 12.9-3.5 17.4l-64 64c-6.2 6.2-16.4 6.2-22.6 0l-64-64c-4.6-4.6-5.9-11.5-3.5-17.4zM384 480L64 480c-35.3 0-64-28.7-64-64l0-64c0-35.3 28.7-64 64-64l320 0c35.3 0 64 28.7 64 64l0 64c0 35.3-28.7 64-64 64zM160 160l0-64-96 0 0 64 96 0z";
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
    exports.faDiagramSuccessor = exports.definition;
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

// src/_entries/faDiagramSuccessor.js
var import_faDiagramSuccessor = __toESM(require_faDiagramSuccessor());
var export_faDiagramSuccessor = import_faDiagramSuccessor.definition;
export {
  export_faDiagramSuccessor as faDiagramSuccessor
};
