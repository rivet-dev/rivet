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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTheta.js
var require_faTheta = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTheta.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "theta";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f69e";
    var svgPathData = "M64 256c0-51 15.9-96 40.2-127.6C128.5 96.9 159.8 80 192 80s63.5 16.9 87.8 48.4C304.1 160 320 205 320 256s-15.9 96-40.2 127.6C255.5 415.1 224.2 432 192 432s-63.5-16.9-87.8-48.4C79.9 352 64 307 64 256zM192 16C135.8 16 87.1 45.7 53.5 89.4C19.9 133.1 0 192.1 0 256s19.9 122.9 53.5 166.6C87.1 466.3 135.8 496 192 496s104.9-29.7 138.5-73.4C364.1 378.9 384 319.9 384 256s-19.9-122.9-53.5-166.6C296.9 45.7 248.2 16 192 16zM128 224c-17.7 0-32 14.3-32 32s14.3 32 32 32l128 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-128 0z";
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
    exports.faTheta = exports.definition;
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

// src/_entries/faTheta.js
var import_faTheta = __toESM(require_faTheta());
var export_faTheta = import_faTheta.definition;
export {
  export_faTheta as faTheta
};
