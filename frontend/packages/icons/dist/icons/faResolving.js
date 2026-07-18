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

// src/node_modules/@fortawesome/free-brands-svg-icons/faResolving.js
var require_faResolving = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faResolving.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "resolving";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f3e7";
    var svgPathData = "M289.2 278.2c46-13.3 49.6-23.5 44-43.4L322 195.5c-6.1-20.9-18.4-28.1-71.1-12.8l-188.2 54.1 28.6 98.6 197.9-57.2zM256.5 8C139.4 8 41.2 88.7 15.2 197.5l221.9-63.9c34.8-10.2 54.2-11.7 79.3-8.2 36.3 6.1 52.7 25 61.4 55.2l10.7 37.8c8.2 28.1 1 50.6-23.5 73.6-19.4 17.4-31.2 24.5-61.4 33.2l-92.6 26.6 220.4 27.1 9.7 34.2-48.1 13.3-286.8-37.3 23 80.2C166 491.3 209.5 504 255.5 504 392.5 504 504 392.6 504 255.7 505 119.4 393.5 8 256.5 8zM46.3 388.6L8 256.8c0 48.5 14.3 93.4 38.3 131.8z";
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
    exports.faResolving = exports.definition;
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

// src/_entries/faResolving.js
var import_faResolving = __toESM(require_faResolving());
var export_faResolving = import_faResolving.definition;
export {
  export_faResolving as faResolving
};
