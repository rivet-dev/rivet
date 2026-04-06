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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChessKingPiece.js
var require_faChessKingPiece = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChessKingPiece.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chess-king-piece";
    var width = 256;
    var height = 512;
    var aliases = ["chess-king-alt"];
    var unicode = "f440";
    var svgPathData = "M144 16c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 16L96 32c-8.8 0-16 7.2-16 16s7.2 16 16 16l16 0 0 32L60.2 96C49.1 96 40 105.1 40 116.2c0 2.5 .5 4.9 1.3 7.3L73.8 208 72 208c-13.3 0-24 10.7-24 24s10.7 24 24 24l4 0L60 384l136 0L180 256l4 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-1.8 0 32.5-84.5c.9-2.3 1.3-4.8 1.3-7.3c0-11.2-9.1-20.2-20.2-20.2L144 96l0-32 16 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-16 0 0-16zM48 416L4.8 473.6C1.7 477.8 0 482.8 0 488c0 13.3 10.7 24 24 24l208 0c13.3 0 24-10.7 24-24c0-5.2-1.7-10.2-4.8-14.4L208 416 48 416z";
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
    exports.faChessKingPiece = exports.definition;
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

// src/_entries/faChessKingAlt.js
var import_faChessKingPiece = __toESM(require_faChessKingPiece());
var export_faChessKingAlt = import_faChessKingPiece.definition;
export {
  export_faChessKingAlt as faChessKingAlt
};
