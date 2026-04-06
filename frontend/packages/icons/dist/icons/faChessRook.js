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

// src/node_modules/@fortawesome/free-solid-svg-icons/faChessRook.js
var require_faChessRook = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faChessRook.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chess-rook";
    var width = 384;
    var height = 512;
    var aliases = [9820];
    var unicode = "f447";
    var svgPathData = "M0 32L0 133.5c0 17 6.7 33.3 18.7 45.3L64 224 64 384 7.8 454.3C2.7 460.6 0 468.4 0 476.5 0 496.1 15.9 512 35.5 512l312.9 0c19.6 0 35.5-15.9 35.5-35.5 0-8.1-2.7-15.9-7.8-22.2l-56.2-70.3 0-160 45.3-45.3c12-12 18.7-28.3 18.7-45.3L384 32c0-17.7-14.3-32-32-32L320 0c-17.7 0-32 14.3-32 32l0 32-48 0 0-32c0-17.7-14.3-32-32-32L176 0c-17.7 0-32 14.3-32 32l0 32-48 0 0-32C96 14.3 81.7 0 64 0L32 0C14.3 0 0 14.3 0 32z";
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
    exports.faChessRook = exports.definition;
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

// src/_entries/faChessRook.js
var import_faChessRook = __toESM(require_faChessRook());
var export_faChessRook = import_faChessRook.definition;
export {
  export_faChessRook as faChessRook
};
