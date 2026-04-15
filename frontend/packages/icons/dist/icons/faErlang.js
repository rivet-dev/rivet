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

// src/node_modules/@fortawesome/free-brands-svg-icons/faErlang.js
var require_faErlang = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faErlang.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "erlang";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f39d";
    var svgPathData = "M87.2 53.5l-87.2 0 0 405 100.4 0c-49.7-52.6-78.8-125.3-78.7-212.1-.1-76.7 24-142.7 65.5-192.9zm238.2 9.7c-45.9 .1-85.1 33.5-89.2 83.2l169.9 0c-1.1-49.7-34.5-83.1-80.7-83.2zm230.7-9.6l.3 0-.1-.1-.2 .1zm.3 0c31.4 42.7 48.7 97.5 46.2 162.7 .5 6 .5 11.7 0 24.1l-372.4 0c-.2 109.7 38.9 194.9 138.6 195.3 68.5-.3 118-51 151.9-106.1l96.4 48.2c-17.4 30.9-36.5 57.8-57.9 80.8l80.8 0 0-405-83.6 0z";
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
    exports.faErlang = exports.definition;
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

// src/_entries/faErlang.js
var import_faErlang = __toESM(require_faErlang());
var export_faErlang = import_faErlang.definition;
export {
  export_faErlang as faErlang
};
