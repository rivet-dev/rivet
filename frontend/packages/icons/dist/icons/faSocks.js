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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSocks.js
var require_faSocks = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSocks.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "socks";
    var width = 512;
    var height = 512;
    var aliases = [129510];
    var unicode = "f696";
    var svgPathData = "M252.8 0L176 0c-26.5 0-48 21.5-48 48l0 16 112 0 0-16c0-17.5 4.7-33.9 12.8-48zM128 112l0 128c0 20.1-9.5 39.1-25.6 51.2l-64 48c-24.2 18.1-38.4 46.6-38.4 76.8 0 53 43 96 96 96 15.4 0 30.5-3.7 44-10.7-17.6-23.9-28-53.4-28-85.3 0-45.3 21.3-88 57.6-115.2l64-48c4-3 6.4-7.8 6.4-12.8l0-128-112 0zm160 0l0 128c0 20.1-9.5 39.1-25.6 51.2l-64 48c-24.2 18.1-38.4 46.6-38.4 76.8 0 53 43 96 96 96 20.8 0 41-6.7 57.6-19.2l115.2-86.4C461 382.2 480 344.3 480 304l0-192-192 0zM480 64l0-16c0-26.5-21.5-48-48-48L336 0c-26.5 0-48 21.5-48 48l0 16 192 0z";
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
    exports.faSocks = exports.definition;
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

// src/_entries/faSocks.js
var import_faSocks = __toESM(require_faSocks());
var export_faSocks = import_faSocks.definition;
export {
  export_faSocks as faSocks
};
