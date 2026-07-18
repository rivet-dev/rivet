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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFan.js
var require_faFan = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFan.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fan";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f863";
    var svgPathData = "M160 144c0-79.5 64.5-144 144-144 8.8 0 16 7.2 16 16l0 152.2c15-5.3 31.2-8.2 48-8.2 79.5 0 144 64.5 144 144 0 8.8-7.2 16-16 16l-152.2 0c5.3 15 8.2 31.2 8.2 48 0 79.5-64.5 144-144 144-8.8 0-16-7.2-16-16l0-152.2c-15 5.3-31.2 8.2-48 8.2-79.5 0-144-64.5-144-144 0-8.8 7.2-16 16-16l152.2 0c-5.3-15-8.2-31.2-8.2-48zm96 144a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faFan = exports.definition;
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

// src/_entries/faFan.js
var import_faFan = __toESM(require_faFan());
var export_faFan = import_faFan.definition;
export {
  export_faFan as faFan
};
