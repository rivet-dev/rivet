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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSquareBinary.js
var require_faSquareBinary = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSquareBinary.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-binary";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e69b";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm144 4c-24.3 0-44 19.7-44 44l0 48c0 24.3 19.7 44 44 44l32 0c24.3 0 44-19.7 44-44l0-48c0-24.3-19.7-44-44-44l-32 0zm-4 44c0-2.2 1.8-4 4-4l32 0c2.2 0 4 1.8 4 4l0 48c0 2.2-1.8 4-4 4l-32 0c-2.2 0-4-1.8-4-4l0-48zm140-44c-11 0-20 9-20 20 0 9.7 6.9 17.7 16 19.6l0 76.4c0 11 9 20 20 20s20-9 20-20l0-96c0-11-9-20-20-20l-16 0zM132 296c0 9.7 6.9 17.7 16 19.6l0 76.4c0 11 9 20 20 20s20-9 20-20l0-96c0-11-9-20-20-20l-16 0c-11 0-20 9-20 20zm96 24l0 48c0 24.3 19.7 44 44 44l32 0c24.3 0 44-19.7 44-44l0-48c0-24.3-19.7-44-44-44l-32 0c-24.3 0-44 19.7-44 44zm44-4l32 0c2.2 0 4 1.8 4 4l0 48c0 2.2-1.8 4-4 4l-32 0c-2.2 0-4-1.8-4-4l0-48c0-2.2 1.8-4 4-4z";
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
    exports.faSquareBinary = exports.definition;
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

// src/_entries/faSquareBinary.js
var import_faSquareBinary = __toESM(require_faSquareBinary());
var export_faSquareBinary = import_faSquareBinary.definition;
export {
  export_faSquareBinary as faSquareBinary
};
