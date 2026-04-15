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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCandyCane.js
var require_faCandyCane = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCandyCane.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "candy-cane";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f786";
    var svgPathData = "M421.8 39.6C452 85.2 456.5 142.9 435.1 192l-165.4 0 41.4-41.4c.8-.8 1.5-1.6 2.2-2.4L421.8 39.6zM268 102.9c-.8 .7-1.6 1.4-2.4 2.2l-28.1 28.1c-25 25-65.5 25-90.5 0s-25-65.5 0-90.5L175 14.6C229.9-40.2 314.5-46.7 376.5-5.6L268 102.9zM125.7 336l80-80 181 0-82.5 82.5 0-2.5-178.5 0zm117 64L141.4 501.3c-25 25-65.5 25-90.5 0s-25-65.5 0-90.5l10.7-10.7 181 0z";
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
    exports.faCandyCane = exports.definition;
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

// src/_entries/faCandyCane.js
var import_faCandyCane = __toESM(require_faCandyCane());
var export_faCandyCane = import_faCandyCane.definition;
export {
  export_faCandyCane as faCandyCane
};
