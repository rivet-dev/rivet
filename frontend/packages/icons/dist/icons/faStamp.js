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

// src/node_modules/@fortawesome/free-solid-svg-icons/faStamp.js
var require_faStamp = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faStamp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "stamp";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5bf";
    var svgPathData = "M312 201.8c0-17.4 9.2-33.2 19.9-47 12.6-16.2 20.1-36.6 20.1-58.8 0-53-43-96-96-96s-96 43-96 96c0 22.1 7.5 42.5 20.1 58.8 10.7 13.8 19.9 29.6 19.9 47 0 29.9-24.3 54.2-54.2 54.2L112 256c-61.9 0-112 50.1-112 112 0 26.5 21.5 48 48 48l416 0c26.5 0 48-21.5 48-48 0-61.9-50.1-112-112-112l-33.8 0c-29.9 0-54.2-24.3-54.2-54.2zM56 464c-13.3 0-24 10.7-24 24s10.7 24 24 24l400 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L56 464z";
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
    exports.faStamp = exports.definition;
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

// src/_entries/faStamp.js
var import_faStamp = __toESM(require_faStamp());
var export_faStamp = import_faStamp.definition;
export {
  export_faStamp as faStamp
};
