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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChessRookPiece.js
var require_faChessRookPiece = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChessRookPiece.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chess-rook-piece";
    var width = 320;
    var height = 512;
    var aliases = ["chess-rook-alt"];
    var unicode = "f448";
    var svgPathData = "M67.4 384L80 264 49.4 233.4c-6-6-9.4-14.1-9.4-22.6L40 112c0-8.8 7.2-16 16-16l24 0c8.8 0 16 7.2 16 16l0 24c0 4.4 3.6 8 8 8l16 0c4.4 0 8-3.6 8-8l0-24c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 24c0 4.4 3.6 8 8 8l16 0c4.4 0 8-3.6 8-8l0-24c0-8.8 7.2-16 16-16l24 0c8.8 0 16 7.2 16 16l0 98.7c0 8.5-3.4 16.6-9.4 22.6L240 264l12.6 120L67.4 384zM136 280.9c0 3.9 3.2 7.1 7.1 7.1l33.8 0c3.9 0 7.1-3.2 7.1-7.1l0-32.9c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 32.9zM20.8 473.6L64 416l192 0 43.2 57.6c3.1 4.2 4.8 9.2 4.8 14.4c0 13.3-10.7 24-24 24L40 512c-13.3 0-24-10.7-24-24c0-5.2 1.7-10.2 4.8-14.4z";
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
    exports.faChessRookPiece = exports.definition;
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

// src/_entries/faChessRookAlt.js
var import_faChessRookPiece = __toESM(require_faChessRookPiece());
var export_faChessRookAlt = import_faChessRookPiece.definition;
export {
  export_faChessRookAlt as faChessRookAlt
};
