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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceFrownSlight.js
var require_faFaceFrownSlight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceFrownSlight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-frown-slight";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e376";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zm91.9-117.5C330 373.8 299.4 352 256 352s-74 21.8-91.9 42.5c-5.8 6.7-15.9 7.4-22.6 1.6s-7.4-15.9-1.6-22.6c22.2-25.7 61-53.5 116.1-53.5s93.8 27.9 116.1 53.5c5.8 6.7 5.1 16.8-1.6 22.6s-16.8 5.1-22.6-1.6zM144.4 208a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm192-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faFaceFrownSlight = exports.definition;
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

// src/_entries/faFaceFrownSlight.js
var import_faFaceFrownSlight = __toESM(require_faFaceFrownSlight());
var export_faFaceFrownSlight = import_faFaceFrownSlight.definition;
export {
  export_faFaceFrownSlight as faFaceFrownSlight
};
