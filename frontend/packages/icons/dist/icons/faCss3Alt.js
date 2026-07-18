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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCss3Alt.js
var require_faCss3Alt = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCss3Alt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "css3-alt";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f38b";
    var svgPathData = "M0 32L34.9 427.8 192 480 349.1 427.8 384 32 0 32zm313.1 80l-4.8 47.3-115.3 49.3-.3 .1 111.5 0-12.8 146.6-98.2 28.7-98.8-29.2-6.4-73.9 48.9 0 3.2 38.3 52.6 13.3 54.7-15.4 3.7-61.6-166.3-.5 0-.1-.2 .1-3.6-46.3 112.1-46.7 6.5-2.7-122.9 0-5.8-47.3 242.2 0z";
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
    exports.faCss3Alt = exports.definition;
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

// src/_entries/faCss3Alt.js
var import_faCss3Alt = __toESM(require_faCss3Alt());
var export_faCss3Alt = import_faCss3Alt.definition;
export {
  export_faCss3Alt as faCss3Alt
};
