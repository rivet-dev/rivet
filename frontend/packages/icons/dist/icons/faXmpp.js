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

// src/node_modules/@fortawesome/free-brands-svg-icons/faXmpp.js
var require_faXmpp = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faXmpp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "xmpp";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e864";
    var svgPathData = "M0 47.2c3.5 137.1 117.1 278.6 252.9 356.7-31.5 25-67.2 44.1-106.2 53.9l0 5.4c56.4-2.3 98.1-20.1 141.3-40.7 65.7 34.6 122.2 39.9 141.4 40.7l0-5.4c-39-9.8-74.7-28.8-106.2-53.8 135.7-78.2 249.3-219.8 252.8-356.8-58.4 25.7-119.1 44.2-180.1 62.3l0 0c3.4 59-20.2 170.4-107.9 262.3-88.4-92.3-111.2-204.1-107.8-262.3l0 0C117.7 92.6 59.3 70.2 0 47.2zM93.9 97l73.6 21.8C165.4 216 226.8 358 341.3 432.2 190.5 376.9 89.9 215.7 93.9 97zm388.3 0c3.3 99-65.6 224.7-171.7 296.4-4.7-4.1-9.3-8.4-13.8-12.7 77-80.2 113.2-186.3 111.9-261.9L482.1 97zM264.5 410.5c3 1.6 5.9 3.2 8.9 4.7-12.5 6.4-25.4 12.1-38.6 17 10.3-6.7 20.2-13.9 29.7-21.7z";
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
    exports.faXmpp = exports.definition;
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

// src/_entries/faXmpp.js
var import_faXmpp = __toESM(require_faXmpp());
var export_faXmpp = import_faXmpp.definition;
export {
  export_faXmpp as faXmpp
};
