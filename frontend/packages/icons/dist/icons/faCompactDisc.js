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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCompactDisc.js
var require_faCompactDisc = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCompactDisc.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "compact-disc";
    var width = 512;
    var height = 512;
    var aliases = [128191, 128192, 128440];
    var unicode = "f51f";
    var svgPathData = "M0 256a256 256 0 1 1 512 0 256 256 0 1 1 -512 0zm256 32a32 32 0 1 1 0-64 32 32 0 1 1 0 64zm-96-32a96 96 0 1 0 192 0 96 96 0 1 0 -192 0zm-56-16c0-32.4 16.3-66.6 42.8-93.2S207.6 104 240 104c13.3 0 24-10.7 24-24s-10.7-24-24-24c-47.9 0-93.7 23.5-127.1 56.9S56 192.1 56 240c0 13.3 10.7 24 24 24s24-10.7 24-24z";
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
    exports.faCompactDisc = exports.definition;
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

// src/_entries/faCompactDisc.js
var import_faCompactDisc = __toESM(require_faCompactDisc());
var export_faCompactDisc = import_faCompactDisc.definition;
export {
  export_faCompactDisc as faCompactDisc
};
