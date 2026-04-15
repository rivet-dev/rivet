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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPoop.js
var require_faPoop = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPoop.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "poop";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f619";
    var svgPathData = "M254.4 6.6c3.5-4.3 9-6.5 14.5-5.7 46.9 6.3 83.1 46.5 83.1 95.1 0 11.2-1.9 22-5.5 32l5.5 0c35.3 0 64 28.7 64 64 0 19.1-8.4 36.3-21.7 48l13.7 0c39.8 0 72 32.2 72 72 0 23.2-11 43.8-28 57 34.1 5.7 60 35.3 60 71 0 39.8-32.2 72-72 72L72 512c-39.8 0-72-32.2-72-72 0-35.7 25.9-65.3 60-71-17-13.2-28-33.8-28-57 0-39.8 32.2-72 72-72l13.7 0c-13.3-11.7-21.7-28.9-21.7-48 0-35.3 28.7-64 64-64l16.2 0c44.1-.1 79.8-35.9 79.8-80 0-9.2-1.5-17.9-4.3-26.1-1.8-5.2-.8-11.1 2.8-15.4z";
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
    exports.faPoop = exports.definition;
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

// src/_entries/faPoop.js
var import_faPoop = __toESM(require_faPoop());
var export_faPoop = import_faPoop.definition;
export {
  export_faPoop as faPoop
};
