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

// src/node_modules/@fortawesome/free-brands-svg-icons/faThreema.js
var require_faThreema = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faThreema.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "threema";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e85f";
    var svgPathData = "M87.4 445.1c18.5 0 33.5 15 33.5 33.4S105.9 512 87.4 512 54 497 54 478.5 69 445.1 87.4 445.1zm109.2 0c18.5 0 33.5 15 33.5 33.4s-15 33.5-33.5 33.5-33.4-15-33.4-33.5 15-33.4 33.4-33.4zm109.3 0c18.5 0 33.5 15 33.5 33.4s-15 33.5-33.5 33.5-33.5-15-33.5-33.5 15-33.4 33.5-33.4zM192.3 .3c109.8 0 199.2 89.3 199.2 199.1S302.2 398.5 192.4 398.5c-40.5 0-78.1-12.1-109.6-32.9l-76.5 24.5 24.8-74C7.3 283.4-6.8 243-6.8 199.4-6.8 89.7 82.5 .3 192.3 .3zm.1 93.7c-31.6 0-57.3 25.8-57.3 57.3l0 26.7-1.8 0c-10 0-18 8-18 18l0 72.3c0 10 8 18 18 18l118.1 0c10 0 18-8 18-18l.1 0 0-72.3c0-10-8.1-18-18-18l-1.8 0 0-26.7c0-31.6-25.8-57.3-57.3-57.3zm-.1 23.5c18.6 0 33.8 15.1 33.8 33.8l0 26.7-67.6 0 0-26.7c0-18.6 15.1-33.8 33.8-33.8z";
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
    exports.faThreema = exports.definition;
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

// src/_entries/faThreema.js
var import_faThreema = __toESM(require_faThreema());
var export_faThreema = import_faThreema.definition;
export {
  export_faThreema as faThreema
};
