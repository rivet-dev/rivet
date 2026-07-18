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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTent.js
var require_faTent = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTent.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tent";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e57d";
    var svgPathData = "M26.9 206.9L3.7 444.9C1.8 463.7 16.6 480 35.5 480l172.7 0c26.5 0 48-21.5 48-48l0-129c0-8.3 6.7-15 15-15 5.5 0 10.6 3 13.2 7.9l86.1 159c8.4 15.5 24.6 25.1 42.2 25.1l64.1 0c18.9 0 33.7-16.3 31.8-35.1L485.6 207.1c-1.9-19.6-12.8-37.3-29.5-47.8L280.9 48.3c-15.7-10-35.8-9.9-51.5 .1L56.1 159.2c-16.5 10.6-27.3 28.2-29.2 47.7z";
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
    exports.faTent = exports.definition;
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

// src/_entries/faTent.js
var import_faTent = __toESM(require_faTent());
var export_faTent = import_faTent.definition;
export {
  export_faTent as faTent
};
