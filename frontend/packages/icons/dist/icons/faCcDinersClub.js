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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCcDinersClub.js
var require_faCcDinersClub = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCcDinersClub.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "cc-diners-club";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f24c";
    var svgPathData = "M239.7 79.9a175.8 175.8 0 1 0 0 351.6 175.8 175.8 0 1 0 0-351.6zM199.8 359.5c-41.7-15.9-71.4-56.4-71.4-103.8s29.7-87.9 71.4-104.1l0 207.9zm79.8 .3l0-208.2c41.7 16.2 71.4 56.7 71.4 104.1s-29.7 87.9-71.4 104.1zM528 32L48 32C21.5 32 0 53.5 0 80L0 432c0 26.5 21.5 48 48 48l480 0c26.5 0 48-21.5 48-48l0-352c0-26.5-21.5-48-48-48zM329.7 448l-90.3 0C133.2 448 45.6 362.5 45.6 257.8 45.6 143.2 133.2 64 239.4 64l90.3 0c105 0 200.7 79.2 200.7 193.8 0 104.7-95.7 190.2-200.7 190.2z";
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
    exports.faCcDinersClub = exports.definition;
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

// src/_entries/faCcDinersClub.js
var import_faCcDinersClub = __toESM(require_faCcDinersClub());
var export_faCcDinersClub = import_faCcDinersClub.definition;
export {
  export_faCcDinersClub as faCcDinersClub
};
