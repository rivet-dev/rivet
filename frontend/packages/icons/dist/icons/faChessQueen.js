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

// src/node_modules/@fortawesome/free-solid-svg-icons/faChessQueen.js
var require_faChessQueen = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faChessQueen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chess-queen";
    var width = 512;
    var height = 512;
    var aliases = [9819];
    var unicode = "f445";
    var svgPathData = "M256 80a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM5.5 185L128 384 71.8 454.3c-5 6.3-7.8 14.1-7.8 22.2 0 19.6 15.9 35.5 35.5 35.5l312.9 0c19.6 0 35.5-15.9 35.5-35.5 0-8.1-2.7-15.9-7.8-22.2L384 384 506.5 185c3.6-5.9 5.5-12.7 5.5-19.6l0-.6c0-20.3-16.5-36.8-36.8-36.8-7.3 0-14.4 2.2-20.4 6.2l-16.9 11.3c-12.7 8.5-29.6 6.8-40.4-4l-34.1-34.1C356.1 100.1 346.2 96 336 96s-20.1 4.1-27.3 11.3l-30.1 30.1c-12.5 12.5-32.8 12.5-45.3 0l-30.1-30.1C196.1 100.1 186.2 96 176 96s-20.1 4.1-27.3 11.3l-34.1 34.1c-10.8 10.8-27.7 12.5-40.4 4L57.3 134.2c-6.1-4-13.2-6.2-20.4-6.2-20.3 0-36.8 16.5-36.8 36.8l0 .6c0 6.9 1.9 13.7 5.5 19.6z";
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
    exports.faChessQueen = exports.definition;
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

// src/_entries/faChessQueen.js
var import_faChessQueen = __toESM(require_faChessQueen());
var export_faChessQueen = import_faChessQueen.definition;
export {
  export_faChessQueen as faChessQueen
};
