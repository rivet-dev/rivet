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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGamepadModern.js
var require_faGamepadModern = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGamepadModern.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "gamepad-modern";
    var width = 640;
    var height = 512;
    var aliases = [127918, 63676, "gamepad-alt"];
    var unicode = "e5a2";
    var svgPathData = "M0 369.4l0 2.8C0 431.7 48.3 480 107.8 480c49.5 0 92.6-33.7 104.6-81.7L216 384l208 0 3.6 14.3c12 48 55.1 81.7 104.6 81.7C591.7 480 640 431.7 640 372.2l0-2.8c0-6.3-.5-12.5-1.6-18.7L605.7 159.8c-8.6-50.2-40.9-93.2-90.3-105.5C469.5 42.9 403.8 32 320 32s-149.5 10.9-195.3 22.3C75.2 66.6 42.9 109.6 34.3 159.8L1.6 350.7C.5 356.9 0 363.1 0 369.4zM432 232a40 40 0 1 1 0 80 40 40 0 1 1 0-80zm24-56a40 40 0 1 1 80 0 40 40 0 1 1 -80 0zm-240-8l0 32 32 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-32 0 0 32c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-32-32 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l32 0 0-32c0-13.3 10.7-24 24-24s24 10.7 24 24z";
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
    exports.faGamepadModern = exports.definition;
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

// src/_entries/faGamepadModern.js
var import_faGamepadModern = __toESM(require_faGamepadModern());
var export_faGamepadModern = import_faGamepadModern.definition;
export {
  export_faGamepadModern as faGamepadModern
};
