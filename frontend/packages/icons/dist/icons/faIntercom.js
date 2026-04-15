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

// src/node_modules/@fortawesome/free-brands-svg-icons/faIntercom.js
var require_faIntercom = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faIntercom.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "intercom";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f7af";
    var svgPathData = "M392 32L56 32C25.1 32 0 57.1 0 88L0 424c0 30.9 25.1 56 56 56l336 0c30.9 0 56-25.1 56-56l0-336c0-30.9-25.1-56-56-56zM283.7 114.1c0-19.8 29.9-19.8 29.9 0l0 199.5c0 19.8-29.9 19.8-29.9 0l0-199.5zm-74.6-7.5c0-19.8 29.9-19.8 29.9 0l0 216.5c0 19.8-29.9 19.8-29.9 0l0-216.5zm-74.7 7.5c0-19.8 29.9-19.8 29.9 0l0 199.5c0 19.8-29.9 19.8-29.9 0l0-199.5zM59.7 144c0-19.8 29.9-19.8 29.9 0l0 134.3c0 19.8-29.9 19.8-29.9 0l0-134.3zM383.1 371.8c-72.8 63-241.7 65.4-318.1 0-15-12.8 4.4-35.5 19.4-22.7 65.9 55.3 216.1 53.9 279.3 0 14.9-12.9 34.3 9.8 19.4 22.7zm5.2-93.5c0 19.8-29.9 19.8-29.9 0l0-134.3c0-19.8 29.9-19.8 29.9 0l0 134.3z";
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
    exports.faIntercom = exports.definition;
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

// src/_entries/faIntercom.js
var import_faIntercom = __toESM(require_faIntercom());
var export_faIntercom = import_faIntercom.definition;
export {
  export_faIntercom as faIntercom
};
