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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChessQueenPiece.js
var require_faChessQueenPiece = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChessQueenPiece.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chess-queen-piece";
    var width = 256;
    var height = 512;
    var aliases = ["chess-queen-alt"];
    var unicode = "f446";
    var svgPathData = "M104 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zM229.9 124.9l-41.5 67.5c11.2 2.1 19.6 11.8 19.6 23.6c0 13.3-10.7 24-24 24l-8 0 19.6 144L60.4 384 80 240l-8 0c-13.3 0-24-10.7-24-24c0-11.8 8.5-21.5 19.6-23.6L26.1 124.9c-3.7-6-3-13.6 1.6-18.9s12.1-6.9 18.6-4.1c3 1.3 6.3 2.1 9.8 2.1c11.8 0 21.6-8.5 23.6-19.7c1.1-6.3 6-11.4 12.3-12.8s12.8 1.1 16.5 6.4C112.8 84 119.9 88 128 88s15.2-4 19.6-10.1c3.7-5.3 10.2-7.8 16.5-6.4s11.1 6.4 12.3 12.8c2 11.2 11.8 19.7 23.6 19.7c3.5 0 6.8-.7 9.8-2.1c6.4-2.9 13.9-1.2 18.6 4.1s5.3 13 1.6 18.9zM208 416l43.2 57.6c3.1 4.2 4.8 9.2 4.8 14.4c0 13.3-10.7 24-24 24L24 512c-13.3 0-24-10.7-24-24c0-5.2 1.7-10.2 4.8-14.4L48 416l160 0z";
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
    exports.faChessQueenPiece = exports.definition;
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

// src/_entries/faChessQueenAlt.js
var import_faChessQueenPiece = __toESM(require_faChessQueenPiece());
var export_faChessQueenAlt = import_faChessQueenPiece.definition;
export {
  export_faChessQueenAlt as faChessQueenAlt
};
