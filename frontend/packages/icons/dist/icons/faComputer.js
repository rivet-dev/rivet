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

// src/node_modules/@fortawesome/free-solid-svg-icons/faComputer.js
var require_faComputer = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faComputer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "computer";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e4e5";
    var svgPathData = "M348.8 32C340.7 46.1 336 62.5 336 80l0 16-272 0 0 224 272 0 0 64-272 0c-35.3 0-64-28.7-64-64L0 96C0 60.7 28.7 32 64 32l284.8 0zM336 432c0 17.5 4.7 33.9 12.8 48L120 480c-13.3 0-24-10.7-24-24s10.7-24 24-24l216 0zM432 32l96 0c26.5 0 48 21.5 48 48l0 352c0 26.5-21.5 48-48 48l-96 0c-26.5 0-48-21.5-48-48l0-352c0-26.5 21.5-48 48-48zm24 64c-13.3 0-24 10.7-24 24s10.7 24 24 24l48 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-48 0zm0 96c-13.3 0-24 10.7-24 24s10.7 24 24 24l48 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-48 0zm56 144a32 32 0 1 0 -64 0 32 32 0 1 0 64 0z";
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
    exports.faComputer = exports.definition;
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

// src/_entries/faComputer.js
var import_faComputer = __toESM(require_faComputer());
var export_faComputer = import_faComputer.definition;
export {
  export_faComputer as faComputer
};
