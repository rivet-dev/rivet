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

// src/node_modules/@fortawesome/free-brands-svg-icons/faExpeditedssl.js
var require_faExpeditedssl = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faExpeditedssl.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "expeditedssl";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f23e";
    var svgPathData = "M256 43.4a212.6 212.6 0 1 0 0 425.2 212.6 212.6 0 1 0 0-425.2zM158.6 176.3c0-53.7 43.7-97.4 97.4-97.4s97.4 43.7 97.4 97.4l0 26.6c0 5-3.9 8.9-8.9 8.9l-17.7 0c-5 0-8.9-3.9-8.9-8.9l0-26.6c0-82.1-124-82.1-124 0l0 26.6c0 5-3.9 8.9-8.9 8.9l-17.7 0c-5 0-8.9-3.9-8.9-8.9l0-26.6 .2 0zM397.7 380c0 9.7-8 17.7-17.7 17.7l-248 0c-9.7 0-17.7-8-17.7-17.7l0-141.7c0-9.7 8-17.7 17.7-17.7l248 0c9.7 0 17.7 8 17.7 17.7l0 141.7zm-248-137.3l0 132.9c0 2.5-1.9 4.4-4.4 4.4l-8.9 0c-2.5 0-4.4-1.9-4.4-4.4l0-132.9c0-2.5 1.9-4.4 4.4-4.4l8.9 0c2.5 0 4.4 1.9 4.4 4.4zm141.7 48.7c0 13-7.2 24.4-17.7 30.4l0 31.6c0 5-3.9 8.9-8.9 8.9l-17.7 0c-5 0-8.9-3.9-8.9-8.9l0-31.6c-10.5-6.1-17.7-17.4-17.7-30.4 0-19.7 15.8-35.4 35.4-35.4s35.5 15.8 35.5 35.4zM256 8a248 248 0 1 0 0 496 248 248 0 1 0 0-496zm0 17.7a230.3 230.3 0 1 1 0 460.6 230.3 230.3 0 1 1 0-460.6z";
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
    exports.faExpeditedssl = exports.definition;
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

// src/_entries/faExpeditedssl.js
var import_faExpeditedssl = __toESM(require_faExpeditedssl());
var export_faExpeditedssl = import_faExpeditedssl.definition;
export {
  export_faExpeditedssl as faExpeditedssl
};
