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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTelescope.js
var require_faTelescope = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTelescope.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "telescope";
    var width = 576;
    var height = 512;
    var aliases = [128301];
    var unicode = "e03e";
    var svgPathData = "M375.7 40c-15.3 8.8-20.6 28.4-11.7 43.7L460 250c8.8 15.3 28.4 20.5 43.7 11.7l55.4-32c15.3-8.8 20.5-28.4 11.7-43.7l-96-166.3C466 4.4 446.4-.8 431.1 8L375.7 40zM11.8 343.6C.4 350.3-3.6 364.9 3.1 376.4l25.9 44.9c6.6 11.5 21.3 15.4 32.8 8.8l44.1-25.5 .8 1.4c8.6 15 27.6 20.4 42.8 12.2l88.4-47.4L193.8 478.9c-5 12.3 .9 26.3 13.2 31.3s26.3-.9 31.3-13.2L304 335.6l65.8 161.4c5 12.3 19 18.2 31.3 13.2s18.2-19 13.2-31.3L346.4 312.5l86.1-46.2-.2-.3-96-166.3c-1-1.8-2-3.6-2.8-5.4L62.4 268.4c-14.4 9.2-19 28.1-10.4 42.9l4 6.9L11.8 343.6z";
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
    exports.faTelescope = exports.definition;
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

// src/_entries/faTelescope.js
var import_faTelescope = __toESM(require_faTelescope());
var export_faTelescope = import_faTelescope.definition;
export {
  export_faTelescope as faTelescope
};
