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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTablePivot.js
var require_faTablePivot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTablePivot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "table-pivot";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e291";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l64 0 0 128L0 160 0 96zm448 64l-288 0 0-128 288 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 192l128 0 0 224 320 0 0-256zm-33.2 86.1c-2.5 6-8.3 9.9-14.8 9.9l-16 0 0 48c0 26.5-21.5 48-48 48l-48 0 0 16c0 6.5-3.9 12.3-9.9 14.8s-12.9 1.1-17.4-3.5l-32-32c-6.2-6.2-6.2-16.4 0-22.6l32-32c4.6-4.6 11.5-5.9 17.4-3.5s9.9 8.3 9.9 14.8l0 16 48 0c8.8 0 16-7.2 16-16l0-48-16 0c-6.5 0-12.3-3.9-14.8-9.9s-1.1-12.9 3.5-17.4l32-32c6.2-6.2 16.4-6.2 22.6 0l32 32c4.6 4.6 5.9 11.5 3.5 17.4z";
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
    exports.faTablePivot = exports.definition;
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

// src/_entries/faTablePivot.js
var import_faTablePivot = __toESM(require_faTablePivot());
var export_faTablePivot = import_faTablePivot.definition;
export {
  export_faTablePivot as faTablePivot
};
