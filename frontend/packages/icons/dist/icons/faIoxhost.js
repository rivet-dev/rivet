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

// src/node_modules/@fortawesome/free-brands-svg-icons/faIoxhost.js
var require_faIoxhost = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faIoxhost.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "ioxhost";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f208";
    var svgPathData = "M616 160l-67.3 0C511.2 70.7 422.9 8 320 8 183 8 72 119 72 256 72 272.4 73.6 288.5 76.7 304L24 304c-13.3 0-24 10.8-24 24 0 13.3 10.7 24 24 24l67.3 0c37.5 89.3 125.8 152 228.7 152 137 0 248-111 248-248 0-16.4-1.6-32.5-4.7-48l52.7 0c13.3 0 24-10.8 24-24 0-13.3-10.7-24-24-24zm-96 96c0 110.5-89.5 200-200 200-75.7 0-141.6-42-175.5-104L424 352c13.3 0 24-10.8 24-24 0-13.3-10.7-24-24-24l-298.2 0c-3.8-15.4-5.8-31.4-5.8-48 0-110.5 89.5-200 200-200 75.7 0 141.6 42 175.5 104L216 160c-13.3 0-24 10.8-24 24 0 13.3 10.7 24 24 24l298.2 0c3.8 15.4 5.8 31.4 5.8 48zM216 232l208 0c13.3 0 24 10.7 24 24 0 13.2-10.7 24-24 24l-208 0c-13.3 0-24-10.7-24-24 0-13.2 10.7-24 24-24z";
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
    exports.faIoxhost = exports.definition;
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

// src/_entries/faIoxhost.js
var import_faIoxhost = __toESM(require_faIoxhost());
var export_faIoxhost = import_faIoxhost.definition;
export {
  export_faIoxhost as faIoxhost
};
