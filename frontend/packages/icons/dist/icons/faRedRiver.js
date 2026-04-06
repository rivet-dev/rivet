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

// src/node_modules/@fortawesome/free-brands-svg-icons/faRedRiver.js
var require_faRedRiver = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faRedRiver.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "red-river";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f3e3";
    var svgPathData = "M353.2 32L94.8 32C42.4 32 0 74.4 0 126.8L0 385.2C0 437.6 42.4 480 94.8 480l258.4 0c52.4 0 94.8-42.4 94.8-94.8l0-258.4C448 74.4 405.6 32 353.2 32zM144.9 200.9l0 56.3c0 27-21.9 48.9-48.9 48.9l0-154.2c0-13.2 10.7-23.9 23.9-23.9l154.2 0c0 27-21.9 48.9-48.9 48.9l-56.3 0c-12.3-.6-24.6 11.6-24 24zm176.3 72l-56.3 0c-12.3-.6-24.6 11.6-24 24l0 56.3c0 27-21.9 48.9-48.9 48.9l0-154.2c0-13.2 10.7-23.9 23.9-23.9l154.2 0c0 27-21.9 48.9-48.9 48.9z";
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
    exports.faRedRiver = exports.definition;
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

// src/_entries/faRedRiver.js
var import_faRedRiver = __toESM(require_faRedRiver());
var export_faRedRiver = import_faRedRiver.definition;
export {
  export_faRedRiver as faRedRiver
};
