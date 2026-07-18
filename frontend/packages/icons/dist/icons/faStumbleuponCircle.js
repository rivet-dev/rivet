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

// src/node_modules/@fortawesome/free-brands-svg-icons/faStumbleuponCircle.js
var require_faStumbleuponCircle = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faStumbleuponCircle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "stumbleupon-circle";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f1a3";
    var svgPathData = "M264 8a248 248 0 1 0 0 496 248 248 0 1 0 0-496zm0 177.5c-9.8 0-17.8 8-17.8 17.8l0 106.9c0 40.9-33.9 73.9-74.9 73.9-41.4 0-74.9-33.5-74.9-74.9l0-46.5 57.3 0 0 45.8c0 10 8 17.8 17.8 17.8s17.8-7.9 17.8-17.8l0-108.4c0-40 34.2-72.1 74.7-72.1 40.7 0 74.7 32.3 74.7 72.6l0 23.7-34.1 10.1-22.9-10.7 0-20.6c.1-9.6-7.9-17.6-17.7-17.6zM431.6 309.1c0 41.4-33.5 74.9-74.9 74.9-41.2 0-74.9-33.2-74.9-74.2l0-46.8 22.9 10.7 34.1-10.1 0 47.1c0 9.8 8 17.6 17.8 17.6s17.8-7.9 17.8-17.6l0-48 57.3 0c-.1 45.9-.1 46.4-.1 46.4z";
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
    exports.faStumbleuponCircle = exports.definition;
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

// src/_entries/faStumbleuponCircle.js
var import_faStumbleuponCircle = __toESM(require_faStumbleuponCircle());
var export_faStumbleuponCircle = import_faStumbleuponCircle.definition;
export {
  export_faStumbleuponCircle as faStumbleuponCircle
};
